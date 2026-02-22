use backend::{AppState, WsMessage, build_app};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tokio::sync::broadcast;

/// Default JWT secret used only when `JWT_SECRET` is not configured.
///
/// This value is intentionally unsafe for production and acts as a local
/// development fallback.
const DEFAULT_JWT_SECRET: &str = "default_secret_key_change_me";

/// Maximum number of PostgreSQL connections in the application pool.
const DB_MAX_CONNECTIONS: u32 = 5;

/// Capacity of the in-process broadcast channel used for WebSocket fan-out.
const WS_CHANNEL_CAPACITY: usize = 100;

/// TCP port exposed by the HTTP server.
const SERVER_PORT: u16 = 3000;

/// Application entry point.
///
/// Loads environment variables, initializes shared state, runs migrations,
/// registers HTTP routes, and starts the Axum server.
#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| DEFAULT_JWT_SECRET.to_string());

    let pool = PgPoolOptions::new()
        .max_connections(DB_MAX_CONNECTIONS)
        .connect(&database_url)
        .await
        .expect("Failed to connect to Postgres");

    // Run startup migrations.
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let (tx, _rx) = broadcast::channel::<WsMessage>(WS_CHANNEL_CAPACITY);

    let state = AppState {
        pool,
        jwt_secret,
        tx,
    };

    let app = build_app(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], SERVER_PORT));
    println!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
