mod models;
mod handlers;
mod middleware;
mod utils;
mod db;

use axum::{
    routing::{get, post},
    middleware as axum_middleware,
    Router,
};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool<Postgres>,
    pub jwt_secret: String,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "default_secret_key_change_me".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to Postgres");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    db::seed_data(&pool).await.expect("Failed to seed data");

    let state = AppState { pool, jwt_secret };

    let auth_routes = Router::new()
        .route("/login", post(handlers::auth::login))
        .route("/register", post(handlers::auth::register))
        .route("/join", post(handlers::auth::join))
        .route("/forgot-password", post(handlers::auth::forgot_password))
        .route("/reset-password", post(handlers::auth::reset_password));

    let user_routes = Router::new()
        .route("/", get(handlers::users::get_users))
        .route("/me/password", axum::routing::patch(handlers::users::update_password))
        .route("/", post(handlers::users::create_user).layer(axum_middleware::from_fn(middleware::admin_only)))
        .route("/{id}", axum::routing::delete(handlers::users::delete_user).layer(axum_middleware::from_fn(middleware::admin_only)))
        .layer(axum_middleware::from_fn_with_state(state.clone(), middleware::auth_middleware));

    let invitation_routes = Router::new()
        .route("/", post(handlers::invitations::create_invitation).layer(axum_middleware::from_fn_with_state(state.clone(), middleware::auth_middleware)))
        .route("/{token}", get(handlers::invitations::get_invitation));

    let task_routes = Router::new()
        .route("/", post(handlers::tasks::create_task))
        .route("/{id}", axum::routing::patch(handlers::tasks::update_task).delete(handlers::tasks::delete_task))
        .layer(axum_middleware::from_fn_with_state(state.clone(), middleware::auth_middleware));

    let report_routes = Router::new()
        .route("/", get(handlers::reports::get_reports).post(handlers::reports::create_report))
        .route("/{id}", axum::routing::patch(handlers::reports::update_report))
        .layer(axum_middleware::from_fn_with_state(state.clone(), middleware::auth_middleware));

    let log_routes = Router::new()
        .route("/", get(handlers::logs::get_logs))
        .layer(axum_middleware::from_fn_with_state(state.clone(), middleware::auth_middleware));

    let app = Router::new()
        .nest("/api/auth", auth_routes)
        .nest("/api/users", user_routes)
        .nest("/api/invitations", invitation_routes)
        .nest("/api/tasks", task_routes)
        .nest("/api/reports", report_routes)
        .nest("/api/logs", log_routes)
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
