use backend::email::{EmailService, SendgridEmailProvider, StdoutEmailProvider};
use backend::{AppState, WsMessage, build_app};
use lambda_http::{Error, run};
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
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

fn default_email_provider() -> String {
    "stdout".to_string()
}

fn default_frontend_url() -> String {
    "http://localhost:5173".to_string()
}

#[derive(Debug, Deserialize, Clone)]
struct Config {
    #[serde(rename = "DATABASE_URL")]
    database_url: String,
    #[serde(rename = "DB_MAX_CONNECTIONS", default = "default_db_max_connections")]
    db_max_connections: u32,
    #[serde(rename = "RUN_MIGRATIONS", default = "default_run_migrations")]
    run_migrations: bool,
    #[serde(
        rename = "DB_IDLE_TIMEOUT_SECONDS",
        default = "default_db_idle_timeout_seconds"
    )]
    db_idle_timeout_seconds: u64,
    #[serde(
        rename = "DB_MAX_LIFETIME_SECONDS",
        default = "default_db_max_lifetime_seconds"
    )]
    db_max_lifetime_seconds: u64,
    #[serde(rename = "JWT_SECRET")]
    jwt_secret: String,
    #[serde(rename = "EMAIL_PROVIDER", default = "default_email_provider")]
    email_provider: String,
    #[serde(rename = "SENDGRID_API_KEY")]
    sendgrid_api_key: Option<String>,
    #[serde(rename = "FRONTEND_URL", default = "default_frontend_url")]
    frontend_url: String,
}

impl Config {
    fn from_env() -> Result<Self, String> {
        envy::from_env::<Config>().map_err(|e| format!("Failed to load config from env: {e}"))
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenvy::dotenv().ok();

    // This initialization runs once per warm Lambda container.
    let config = Config::from_env().expect("configuration error");

    let pool = PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .idle_timeout(Some(Duration::from_secs(config.db_idle_timeout_seconds)))
        .max_lifetime(Some(Duration::from_secs(config.db_max_lifetime_seconds)))
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to Postgres");

    if config.run_migrations {
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");
    }

    let (tx, _rx) = broadcast::channel::<WsMessage>(WS_CHANNEL_CAPACITY);

    let email_service: Arc<dyn EmailService> = match config.email_provider.as_str() {
        "stdout" => Arc::new(StdoutEmailProvider::new(config.frontend_url.clone())),
        "sendgrid" => {
            let api_key = config
                .sendgrid_api_key
                .clone()
                .expect("SENDGRID_API_KEY must be set when EMAIL_PROVIDER=sendgrid");
            Arc::new(SendgridEmailProvider::new(
                api_key,
                config.frontend_url.clone(),
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
    run(app).await
}
