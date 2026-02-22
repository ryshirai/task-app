use axum::http::StatusCode;
use axum_test::TestServer;
use backend::{AppState, WsMessage, build_app};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::broadcast;

const FALLBACK_DATABASE_URL: &str = "postgres://postgres:postgres@localhost:5432/postgres";
const TEST_JWT_SECRET: &str = "test-jwt-secret";

async fn build_test_server() -> TestServer {
    let pool = if let Ok(database_url) = std::env::var("DATABASE_URL") {
        match PgPoolOptions::new().max_connections(1).connect(&database_url).await {
            Ok(pool) => pool,
            Err(_) => PgPoolOptions::new()
                .max_connections(1)
                .connect_lazy(&database_url)
                .expect("invalid DATABASE_URL format"),
        }
    } else {
        PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(FALLBACK_DATABASE_URL)
            .expect("invalid fallback database URL")
    };

    let (tx, _rx) = broadcast::channel::<WsMessage>(10);
    let state = AppState {
        pool,
        jwt_secret: TEST_JWT_SECRET.to_string(),
        tx,
    };

    TestServer::new(build_app(state)).expect("failed to build test server")
}

#[tokio::test]
async fn get_login_returns_method_not_allowed() {
    let server = build_test_server().await;
    let response = server.get("/api/auth/login").await;

    assert_eq!(response.status_code(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn post_register_with_invalid_username_returns_bad_request() {
    let server = build_test_server().await;
    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "organization_name": "Acme Inc",
            "admin_name": "Admin User",
            "username": "invalid username!",
            "email": "admin@example.com",
            "password": "secret123",
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}
