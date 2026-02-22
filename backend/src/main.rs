use backend::email::{EmailService, SesEmailProvider, StdoutEmailProvider};
use backend::{AppState, WsMessage, build_app};
use serde::Deserialize;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

/// Capacity of the in-process broadcast channel used for WebSocket fan-out.
const WS_CHANNEL_CAPACITY: usize = 100;

fn default_db_max_connections() -> u32 {
    1
}

fn default_run_migrations() -> bool {
    false
}

fn default_db_idle_timeout_seconds() -> u64 {
    30
}

fn default_db_max_lifetime_seconds() -> u64 {
    1800
}

fn default_port() -> u16 {
    3000
}

fn default_email_provider() -> String {
    "stdout".to_string()
}

#[derive(Debug, Deserialize, Clone)]
struct Config {
    #[serde(rename = "DATABASE_URL", alias = "database_url")]
    database_url: String,
    #[serde(rename = "DOMAIN", alias = "domain")]
    domain: String,
    #[serde(rename = "DB_MAX_CONNECTIONS", alias = "db_max_connections", default = "default_db_max_connections")]
    db_max_connections: u32,
    #[serde(rename = "RUN_MIGRATIONS", alias = "run_migrations", default = "default_run_migrations")]
    run_migrations: bool,
    #[serde(
        rename = "DB_IDLE_TIMEOUT_SECONDS",
        alias = "db_idle_timeout_seconds",
        default = "default_db_idle_timeout_seconds"
    )]
    db_idle_timeout_seconds: u64,
    #[serde(
        rename = "DB_MAX_LIFETIME_SECONDS",
        alias = "db_max_lifetime_seconds",
        default = "default_db_max_lifetime_seconds"
    )]
    db_max_lifetime_seconds: u64,
    #[serde(rename = "JWT_SECRET", alias = "jwt_secret")]
    jwt_secret: String,
    #[serde(rename = "PORT", alias = "port", default = "default_port")]
    port: u16,
    #[serde(rename = "EMAIL_PROVIDER", alias = "email_provider", default = "default_email_provider")]
    email_provider: String,
    #[serde(rename = "EMAIL_FROM_ADDRESS", alias = "email_from_address")]
    email_from_address: Option<String>,
    #[serde(rename = "AWS_REGION", alias = "aws_region")]
    _aws_region: Option<String>,
    #[serde(rename = "FRONTEND_URL", alias = "frontend_url")]
    frontend_url: Option<String>,
}

impl Config {
    fn from_env() -> Result<Self, String> {
        match envy::from_env::<Config>() {
            Ok(config) => Ok(config),
            Err(e) => {
                let keys: Vec<String> = std::env::vars().map(|(k, _)| k).collect();
                Err(format!("{} (Available keys: {:?})", e, keys))
            }
        }
    }

    fn get_email_from_address(&self) -> String {
        self.email_from_address
            .clone()
            .unwrap_or_else(|| format!("no-reply@{}", self.domain))
    }

    fn get_frontend_url(&self) -> String {
        self.frontend_url
            .clone()
            .unwrap_or_else(|| format!("https://{}", self.domain))
    }
}

/// Application entry point.
///
/// Loads environment variables, initializes shared state, runs migrations,
/// registers HTTP routes, and starts the Axum server.
#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config = Config::from_env().expect("configuration error");

    let connect_options = PgConnectOptions::from_str(&config.database_url)
        .expect("Invalid DATABASE_URL")
        .statement_cache_capacity(0);

    let pool = PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .idle_timeout(Some(Duration::from_secs(config.db_idle_timeout_seconds)))
        .max_lifetime(Some(Duration::from_secs(config.db_max_lifetime_seconds)))
        .connect_with(connect_options)
        .await
        .expect("Failed to connect to Postgres");

    if config.run_migrations {
        // Run startup migrations only when explicitly enabled.
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");
    }

    let (tx, _rx) = broadcast::channel::<WsMessage>(WS_CHANNEL_CAPACITY);

    let from_email = config.get_email_from_address();
    let frontend_url = config.get_frontend_url();

    let email_service: Arc<dyn EmailService> = match config.email_provider.as_str() {
        "stdout" => Arc::new(StdoutEmailProvider::new(frontend_url)),
        "ses" => {
            let shared_config = aws_config::load_from_env().await;
            let ses_client = aws_sdk_sesv2::Client::new(&shared_config);
            Arc::new(SesEmailProvider::new(
                ses_client,
                frontend_url,
                from_email,
            ))
        }
        other => panic!("Unsupported EMAIL_PROVIDER: {other}"),
    };

    let state = AppState {
        pool,
        jwt_secret: config.jwt_secret,
        tx,
        email_service,
    };

    let app = build_app(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    println!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
