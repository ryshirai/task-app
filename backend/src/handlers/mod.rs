pub mod auth;
pub mod invitations;
pub mod logs;
pub mod notifications;
pub mod reports;
pub mod tasks;
pub mod users;
pub mod ws;

use sqlx::{Pool, Postgres};

pub async fn log_activity(
    pool: &Pool<Postgres>,
    org_id: i32,
    user_id: i32,
    action: &str,
    target_type: &str,
    target_id: Option<i32>,
    details: Option<String>,
) {
    let _ = sqlx::query(
        "INSERT INTO activity_logs (organization_id, user_id, action, target_type, target_id, details) VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(org_id)
    .bind(user_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(details)
    .execute(pool)
    .await;
}

pub async fn notify_user(
    pool: &Pool<Postgres>,
    organization_id: i32,
    user_id: i32,
    title: &str,
    body: Option<&str>,
    category: &str,
    target_type: Option<&str>,
    target_id: Option<i32>,
) {
    let _ = sqlx::query(
        "INSERT INTO notifications (organization_id, user_id, title, body, category, target_type, target_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(organization_id)
    .bind(user_id)
    .bind(title)
    .bind(body)
    .bind(category)
    .bind(target_type)
    .bind(target_id)
    .execute(pool)
    .await;
}
