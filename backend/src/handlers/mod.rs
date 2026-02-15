pub mod auth;
pub mod users;
pub mod tasks;
pub mod reports;
pub mod invitations;
pub mod logs;
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
