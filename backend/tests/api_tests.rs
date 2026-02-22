use axum::http::StatusCode;
use axum_test::TestServer;
use backend::email::StdoutEmailProvider;
use backend::{AppState, WsMessage, build_app};
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{OnceCell, broadcast};

const FALLBACK_DATABASE_URL: &str = "postgres://postgres:postgres@localhost:5432/postgres";
const TEST_JWT_SECRET: &str = "test-jwt-secret";
static TEST_DB_INIT: OnceCell<()> = OnceCell::const_new();
static UNIQUE_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TestContext {
    server: TestServer,
    pool: PgPool,
}

fn build_app_with_pool(pool: PgPool) -> TestServer {
    let (tx, _rx) = broadcast::channel::<WsMessage>(10);
    let state = AppState {
        pool,
        jwt_secret: TEST_JWT_SECRET.to_string(),
        tx,
        email_service: Arc::new(StdoutEmailProvider::new("http://localhost:5173".to_string())),
    };

    TestServer::new(build_app(state)).expect("failed to build test server")
}

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

    build_app_with_pool(pool)
}

async fn build_test_context() -> TestContext {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| FALLBACK_DATABASE_URL.into());
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("DB-required integration tests need a reachable PostgreSQL (set DATABASE_URL)");
    let migrate_pool = pool.clone();

    TEST_DB_INIT
        .get_or_init(|| async move {
            sqlx::migrate!("./migrations")
                .run(&migrate_pool)
                .await
                .expect("failed to run migrations for integration tests");
        })
        .await;

    let server = build_app_with_pool(pool.clone());
    TestContext { server, pool }
}

fn unique_suffix() -> u128 {
    let epoch_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_millis();
    let increment = UNIQUE_COUNTER.fetch_add(1, Ordering::Relaxed) as u128;
    (epoch_millis << 16) | (increment & 0xFFFF)
}

fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("password hashing should succeed")
        .to_string()
}

async fn create_organization(pool: &PgPool, suffix: u128) -> i32 {
    sqlx::query_scalar("INSERT INTO organizations (name) VALUES ($1) RETURNING id")
        .bind(format!("Role Test Org {suffix}"))
        .fetch_one(pool)
        .await
        .expect("organization creation should succeed")
}

async fn create_user_in_db(
    pool: &PgPool,
    organization_id: i32,
    suffix: u128,
    username_prefix: &str,
    role: &str,
) -> (i32, String) {
    let username = format!("{username_prefix}_{suffix}");
    let password_hash = hash_password("secret123");
    let user_id = sqlx::query_scalar(
        "INSERT INTO users (organization_id, name, username, email, password_hash, role)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id",
    )
    .bind(organization_id)
    .bind(format!("{username_prefix} User"))
    .bind(&username)
    .bind(format!("{username}@example.com"))
    .bind(password_hash)
    .bind(role)
    .fetch_one(pool)
    .await
    .expect("user creation should succeed");

    (user_id, username)
}

async fn login(server: &TestServer, username: &str, password: &str) -> String {
    let response = server
        .post("/api/auth/login")
        .json(&json!({
            "username": username,
            "password": password,
        }))
        .await;

    assert_eq!(
        response.status_code(),
        StatusCode::OK,
        "login should succeed for existing user"
    );

    let body: serde_json::Value = response.json();
    body["token"]
        .as_str()
        .expect("login response should include token")
        .to_string()
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

#[tokio::test]
async fn admin_can_update_member_role() {
    let context = build_test_context().await;
    let suffix = unique_suffix();
    let organization_id = create_organization(&context.pool, suffix).await;
    let (_, admin_username) =
        create_user_in_db(&context.pool, organization_id, suffix, "admin", "admin").await;
    let (member_id, _) =
        create_user_in_db(&context.pool, organization_id, suffix + 1, "member", "member").await;
    let admin_token = login(&context.server, &admin_username, "secret123").await;

    let response = context
        .server
        .put(&format!("/api/users/{member_id}/role"))
        .add_header("Authorization", format!("Bearer {admin_token}"))
        .json(&json!({ "role": "admin" }))
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
}

#[tokio::test]
async fn member_cannot_update_role() {
    let context = build_test_context().await;
    let suffix = unique_suffix();
    let organization_id = create_organization(&context.pool, suffix).await;
    let (actor_member_id, actor_username) =
        create_user_in_db(&context.pool, organization_id, suffix, "actor", "member").await;
    let (target_member_id, _) =
        create_user_in_db(&context.pool, organization_id, suffix + 1, "target", "member").await;

    let actor_token = login(&context.server, &actor_username, "secret123").await;
    assert!(actor_member_id != target_member_id);

    let response = context
        .server
        .put(&format!("/api/users/{target_member_id}/role"))
        .add_header("Authorization", format!("Bearer {actor_token}"))
        .json(&json!({ "role": "admin" }))
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn user_cannot_update_own_role() {
    let context = build_test_context().await;
    let suffix = unique_suffix();
    let organization_id = create_organization(&context.pool, suffix).await;
    let (admin_id, admin_username) =
        create_user_in_db(&context.pool, organization_id, suffix, "admin_self", "admin").await;
    let admin_token = login(&context.server, &admin_username, "secret123").await;

    let response = context
        .server
        .put(&format!("/api/users/{admin_id}/role"))
        .add_header("Authorization", format!("Bearer {admin_token}"))
        .json(&json!({ "role": "member" }))
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn middleware_reflects_instant_role() {
    let context = build_test_context().await;
    let suffix = unique_suffix();
    let organization_id = create_organization(&context.pool, suffix).await;
    let (member_id, member_username) =
        create_user_in_db(&context.pool, organization_id, suffix, "middleware_member", "member")
            .await;

    let member_token = login(&context.server, &member_username, "secret123").await;

    let before_response = context
        .server
        .post("/api/invitations")
        .add_header("Authorization", format!("Bearer {member_token}"))
        .json(&json!({ "role": "user" }))
        .await;
    assert_eq!(before_response.status_code(), StatusCode::FORBIDDEN);

    sqlx::query("UPDATE users SET role = 'admin' WHERE id = $1")
        .bind(member_id)
        .execute(&context.pool)
        .await
        .expect("direct role update should succeed");

    let after_response = context
        .server
        .post("/api/invitations")
        .add_header("Authorization", format!("Bearer {member_token}"))
        .json(&json!({ "role": "user" }))
        .await;
    assert_eq!(after_response.status_code(), StatusCode::CREATED);
}
