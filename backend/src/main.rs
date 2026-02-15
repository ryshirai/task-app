mod db;
mod handlers;
mod middleware;
mod models;
mod utils;

use axum::{
    Router, middleware as axum_middleware,
    routing::{get, post},
};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

use tokio::sync::broadcast;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct WsMessage {
    pub organization_id: i32,
    pub event: String,
    pub payload: serde_json::Value,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool<Postgres>,
    pub jwt_secret: String,
    pub tx: broadcast::Sender<WsMessage>,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let jwt_secret =
        std::env::var("JWT_SECRET").unwrap_or_else(|_| "default_secret_key_change_me".to_string());

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

    let (tx, _rx) = broadcast::channel(100);

    let state = AppState {
        pool,
        jwt_secret,
        tx,
    };

    let auth_routes = Router::new()
        .route("/login", post(handlers::auth::login))
        .route("/register", post(handlers::auth::register))
        .route("/join", post(handlers::auth::join))
        .route("/forgot-password", post(handlers::auth::forgot_password))
        .route("/reset-password", post(handlers::auth::reset_password));

    let user_routes = Router::new()
        .route("/", get(handlers::users::get_users))
        .route(
            "/me/password",
            axum::routing::patch(handlers::users::update_password),
        )
        .route(
            "/",
            post(handlers::users::create_user)
                .layer(axum_middleware::from_fn(middleware::admin_only)),
        )
        .route(
            "/{id}",
            axum::routing::delete(handlers::users::delete_user)
                .layer(axum_middleware::from_fn(middleware::admin_only)),
        )
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth_middleware,
        ));

    let invitation_routes = Router::new()
        .route(
            "/",
            post(handlers::invitations::create_invitation).layer(
                axum_middleware::from_fn_with_state(state.clone(), middleware::auth_middleware),
            ),
        )
        .route("/{token}", get(handlers::invitations::get_invitation));

    let task_routes = Router::new()
        .route("/", get(handlers::tasks::get_tasks).post(handlers::tasks::create_task))
        .route("/time-logs", post(handlers::tasks::add_time_log))
        .route(
            "/time-logs/{id}",
            axum::routing::patch(handlers::tasks::update_time_log)
                .delete(handlers::tasks::delete_time_log),
        )
        .route(
            "/report",
            get(handlers::tasks::get_task_report)
                .layer(axum_middleware::from_fn(middleware::admin_only)),
        )
        .route(
            "/report/export",
            get(handlers::tasks::export_task_report)
                .layer(axum_middleware::from_fn(middleware::admin_only)),
        )
        .route(
            "/{id}",
            axum::routing::patch(handlers::tasks::update_task).delete(handlers::tasks::delete_task),
        )
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth_middleware,
        ));

    let report_routes = Router::new()
        .route(
            "/",
            get(handlers::reports::get_reports).post(handlers::reports::create_report),
        )
        .route(
            "/{id}",
            get(handlers::reports::get_report).patch(handlers::reports::update_report),
        )
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth_middleware,
        ));

    let log_routes = Router::new()
        .route("/export", get(handlers::logs::export_logs))
        .route("/", get(handlers::logs::get_logs))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth_middleware,
        ));

    let notification_routes = Router::new()
        .route("/", get(handlers::notifications::get_notifications))
        .route(
            "/read-all",
            axum::routing::patch(handlers::notifications::mark_all_as_read),
        )
        .route(
            "/{id}/read",
            axum::routing::patch(handlers::notifications::mark_as_read),
        )
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth_middleware,
        ));

    let analytics_routes = Router::new()
        .route(
            "/personal",
            get(handlers::analytics::get_personal_analytics),
        )
        .route("/users/{id}", get(handlers::analytics::get_user_analytics))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth_middleware,
        ));

    let app = Router::new()
        .route(
            "/ws",
            get(handlers::ws::ws_handler).layer(axum_middleware::from_fn_with_state(
                state.clone(),
                middleware::auth_middleware,
            )),
        )
        .nest("/api/auth", auth_routes)
        .nest("/api/users", user_routes)
        .nest("/api/invitations", invitation_routes)
        .nest("/api/tasks", task_routes)
        .nest("/api/reports", report_routes)
        .nest("/api/logs", log_routes)
        .nest("/api/notifications", notification_routes)
        .nest("/api/analytics", analytics_routes)
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
