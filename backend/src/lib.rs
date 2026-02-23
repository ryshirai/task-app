pub mod email;
pub mod models;
mod utils;

#[path = "handlers/analytics.rs"]
mod analytics;
#[path = "handlers/auth.rs"]
mod auth;
#[path = "handlers/groups.rs"]
mod groups;
#[path = "handlers/invitations.rs"]
mod invitations;
#[path = "handlers/logs.rs"]
mod logs;
#[path = "handlers/notifications.rs"]
mod notifications;
#[path = "handlers/reports.rs"]
mod reports;
#[path = "handlers/tasks.rs"]
mod tasks;
#[path = "handlers/users.rs"]
mod users;
#[path = "handlers/ws.rs"]
mod ws;

use serde::Serialize;
use std::sync::Arc;
use worker::*;

fn read_optional_env(env: &Env, key: &str) -> Option<String> {
    env.secret(key)
        .ok()
        .map(|v| v.to_string())
        .or_else(|| env.var(key).ok().map(|v| v.to_string()))
}

fn normalize_origin(origin: &str) -> String {
    origin.trim().trim_end_matches('/').to_string()
}

fn cors_origin(env: &Env, request_origin: Option<&str>) -> Option<String> {
    let mut whitelist = vec!["http://localhost:5173".to_string()];
    if let Some(frontend_url) = read_optional_env(env, "FRONTEND_URL") {
        whitelist.push(frontend_url);
    }

    let request_origin = request_origin.map(normalize_origin)?;
    let is_allowed = whitelist
        .into_iter()
        .map(|origin| normalize_origin(&origin))
        .any(|origin| origin == request_origin);

    if is_allowed {
        Some(request_origin)
    } else {
        None
    }
}

fn with_cors(mut response: Response, env: &Env, request_origin: Option<&str>) -> Result<Response> {
    let headers = response.headers_mut();
    if let Some(origin) = cors_origin(env, request_origin) {
        headers.set("Access-Control-Allow-Origin", &origin)?;
    }
    let vary_value = headers.get("Vary")?.unwrap_or_default();
    let has_origin_vary = vary_value
        .split(',')
        .any(|part| part.trim().eq_ignore_ascii_case("Origin"));
    if has_origin_vary {
        headers.set("Vary", &vary_value)?;
    } else if vary_value.trim().is_empty() {
        headers.set("Vary", "Origin")?;
    } else {
        headers.set("Vary", &format!("{}, Origin", vary_value))?;
    }
    headers.set(
        "Access-Control-Allow-Methods",
        "GET, POST, PUT, PATCH, DELETE, OPTIONS",
    )?;
    headers.set("Access-Control-Allow-Headers", "Content-Type, Authorization")?;
    headers.set("Access-Control-Max-Age", "86400")?;
    Ok(response)
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<D1Database>,
    pub jwt_secret: String,
    pub email_service: Arc<dyn email::EmailService>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
}

#[event(fetch)]
pub async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    #[cfg(debug_assertions)]
    console_log!("fetch: {} {}", req.method().to_string(), req.path());
    let request_origin = req.headers().get("Origin").ok().flatten();

    if req.method() == Method::Options {
        return with_cors(Response::ok("")?, &env, request_origin.as_deref());
    }

    let result: Result<Response> = async {
        let db = Arc::new(env.d1("DB")?);
        let jwt_secret = match env.secret("JWT_SECRET") {
            Ok(secret) => secret.to_string(),
            Err(err) => {
                let environment = read_optional_env(&env, "ENVIRONMENT")
                    .or_else(|| read_optional_env(&env, "NODE_ENV"))
                    .unwrap_or_else(|| "development".to_string());

                if environment.eq_ignore_ascii_case("production") {
                    console_error!("JWT_SECRET is missing in production: {}", err);
                    return Response::error(
                        "Server misconfiguration: JWT_SECRET is missing",
                        500,
                    );
                }

                #[cfg(debug_assertions)]
                {
                    console_error!(
                        "JWT_SECRET is missing (environment={}); using insecure default secret in debug build",
                        environment
                    );
                    "insecure-default-secret".to_string()
                }

                #[cfg(not(debug_assertions))]
                console_log!(
                    "JWT_SECRET is missing (environment={}); refusing startup in non-debug build",
                    environment
                );
                #[cfg(not(debug_assertions))]
                return Response::error(
                    "Server misconfiguration: JWT_SECRET is missing",
                    500,
                );
            }
        };
        let frontend_url = read_optional_env(&env, "FRONTEND_URL")
            .unwrap_or_else(|| "https://example.com".to_string());
        let from_email = read_optional_env(&env, "EMAIL_FROM_ADDRESS")
            .unwrap_or_else(|| "no-reply@example.com".to_string());
        let resend_api_key = read_optional_env(&env, "RESEND_API_KEY");

        let email_service: Arc<dyn email::EmailService> = if let Some(api_key) = resend_api_key {
            #[cfg(debug_assertions)]
            console_log!("email provider: resend");
            Arc::new(email::ResendEmailProvider::new(
                frontend_url,
                from_email,
                api_key,
            ))
        } else {
            #[cfg(debug_assertions)]
            console_log!("email provider: stdout");
            Arc::new(email::StdoutEmailProvider::new(frontend_url))
        };

        let state = AppState {
            db,
            jwt_secret,
            email_service,
        };

        Router::with_data(state)
            .get_async("/", |_req, _ctx| async move {
                Response::ok("GlanceFlow API is running")
            })
            .get_async("/ping", |_req, _ctx| async move { Response::ok("Pong") })
            .get_async("/health", |_req, _ctx| async move {
                Response::from_json(&HealthResponse { status: "ok" })
            })
            .post_async("/api/auth/login", auth::login)
            .post_async("/api/auth/register", auth::register)
            .post_async("/api/auth/join", auth::join)
            .post_async("/api/auth/forgot-password", auth::forgot_password)
            .post_async("/api/auth/reset-password", auth::reset_password)
            .post_async("/api/auth/verify-email", auth::verify_email)
            .post_async("/api/invitations", invitations::create_invitation)
            .get_async("/api/invitations/:token", invitations::get_invitation)
            .get_async("/api/tasks", tasks::get_tasks)
            .post_async("/api/tasks", tasks::create_task)
            .post_async("/api/tasks/time-logs", tasks::add_time_log)
            .patch_async("/api/tasks/time-logs/:id", tasks::update_time_log)
            .delete_async("/api/tasks/time-logs/:id", tasks::delete_time_log)
            .get_async("/api/tasks/report", tasks::get_task_report)
            .get_async("/api/tasks/report/export", tasks::export_task_report)
            .patch_async("/api/tasks/:id", tasks::update_task)
            .delete_async("/api/tasks/:id", tasks::delete_task)
            .get_async("/api/reports", reports::get_reports)
            .post_async("/api/reports", reports::create_report)
            .get_async("/api/reports/:id", reports::get_report)
            .patch_async("/api/reports/:id", reports::update_report)
            .get_async("/api/logs", logs::get_logs)
            .get_async("/api/logs/export", logs::export_logs)
            .get_async("/api/notifications", notifications::get_notifications)
            .patch_async(
                "/api/notifications/read-all",
                notifications::mark_all_as_read,
            )
            .patch_async("/api/notifications/:id/read", notifications::mark_as_read)
            .get_async("/api/analytics/personal", analytics::get_personal_analytics)
            .get_async("/api/analytics/users/:id", analytics::get_user_analytics)
            .get_async("/api/users", users::get_users)
            .post_async("/api/users", users::create_user)
            .patch_async("/api/users/me/password", users::update_password)
            .patch_async("/api/users/me/email", users::update_email)
            .put_async("/api/users/:id/role", users::update_user_role)
            .delete_async("/api/users/:id", users::delete_user)
            .get_async("/api/display-groups", groups::get_display_groups)
            .post_async("/api/display-groups", groups::create_display_group)
            .patch_async("/api/display-groups/:id", groups::update_display_group)
            .delete_async("/api/display-groups/:id", groups::delete_display_group)
            .get_async("/ws", ws::ws_handler)
            .run(req, env.clone())
            .await
    }
    .await;

    match result {
        Ok(response) => with_cors(response, &env, request_origin.as_deref()),
        Err(err) => {
            console_error!("request failed: {:?}", err);
            let response = Response::error("Internal Server Error", 500).or_else(
                |response_err| {
                    console_error!("failed to build error response: {:?}", response_err);
                    Response::from_json(&ErrorResponse {
                        error: "Internal Server Error",
                    })
                    .map(|response| response.with_status(500))
                },
            )?;
            with_cors(response, &env, request_origin.as_deref())
        }
    }
}
