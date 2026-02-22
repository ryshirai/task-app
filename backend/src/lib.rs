mod handlers;
mod middleware;
mod models;
mod utils;

use axum::{
    Router, middleware as axum_middleware,
    routing::{get, post},
};
use sqlx::{Pool, Postgres};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;

/// Message broadcast to WebSocket subscribers.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct WsMessage {
    /// Tenant boundary for the event.
    pub organization_id: i32,
    /// Event type identifier consumed by clients.
    pub event: String,
    /// Event payload as arbitrary JSON.
    pub payload: serde_json::Value,
}

/// Shared application state injected into handlers and middleware.
#[derive(Clone)]
pub struct AppState {
    /// PostgreSQL connection pool.
    pub pool: Pool<Postgres>,
    /// Secret key used to sign JWT tokens.
    pub jwt_secret: String,
    /// Broadcast sender for real-time events.
    pub tx: broadcast::Sender<WsMessage>,
}

/// Builds authentication-related routes.
fn build_auth_routes() -> Router<AppState> {
    Router::new()
        .route("/login", post(handlers::auth::login))
        .route("/register", post(handlers::auth::register))
        .route("/join", post(handlers::auth::join))
        .route("/forgot-password", post(handlers::auth::forgot_password))
        .route("/reset-password", post(handlers::auth::reset_password))
}

/// Builds user management routes protected by authentication middleware.
fn build_user_routes(state: &AppState) -> Router<AppState> {
    Router::new()
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
        ))
}

/// Builds invitation routes.
fn build_invitation_routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/",
            post(handlers::invitations::create_invitation).layer(
                axum_middleware::from_fn_with_state(state.clone(), middleware::auth_middleware),
            ),
        )
        .route("/{token}", get(handlers::invitations::get_invitation))
}

/// Builds task and task-time-log routes protected by authentication.
fn build_task_routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(handlers::tasks::get_tasks).post(handlers::tasks::create_task),
        )
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
        ))
}

/// Builds daily report CRUD routes protected by authentication.
fn build_report_routes(state: &AppState) -> Router<AppState> {
    Router::new()
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
        ))
}

/// Builds activity log routes protected by authentication.
fn build_log_routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/export", get(handlers::logs::export_logs))
        .route("/", get(handlers::logs::get_logs))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth_middleware,
        ))
}

/// Builds notification routes protected by authentication.
fn build_notification_routes(state: &AppState) -> Router<AppState> {
    Router::new()
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
        ))
}

/// Builds analytics routes protected by authentication.
fn build_analytics_routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/personal",
            get(handlers::analytics::get_personal_analytics),
        )
        .route("/users/{id}", get(handlers::analytics::get_user_analytics))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth_middleware,
        ))
}

/// Builds display-group routes protected by authentication.
fn build_display_group_routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(handlers::groups::get_display_groups).post(handlers::groups::create_display_group),
        )
        .route(
            "/{id}",
            axum::routing::patch(handlers::groups::update_display_group)
                .delete(handlers::groups::delete_display_group),
        )
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth_middleware,
        ))
}

/// Builds the API subtree under `/api`.
fn build_api_routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .nest("/auth", build_auth_routes())
        .nest("/users", build_user_routes(state))
        .nest("/invitations", build_invitation_routes(state))
        .nest("/tasks", build_task_routes(state))
        .nest("/reports", build_report_routes(state))
        .nest("/logs", build_log_routes(state))
        .nest("/notifications", build_notification_routes(state))
        .nest("/analytics", build_analytics_routes(state))
        .nest("/display-groups", build_display_group_routes(state))
}

/// Builds the top-level Axum application router.
pub fn build_app(state: AppState) -> Router {
    Router::new()
        .route(
            "/ws",
            get(handlers::ws::ws_handler).layer(axum_middleware::from_fn_with_state(
                state.clone(),
                middleware::auth_middleware,
            )),
        )
        .nest("/api", build_api_routes(&state))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
