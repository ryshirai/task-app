use axum::{
    extract::State,
    http::StatusCode,
    Json, Extension,
};
use crate::models::*;
use crate::AppState;

pub async fn get_logs(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<ActivityLog>>, (StatusCode, String)> {
    let logs = sqlx::query_as::<_, ActivityLog>(
        "SELECT l.*, u.name as user_name FROM activity_logs l 
         JOIN users u ON l.user_id = u.id 
         WHERE l.organization_id = $1
         ORDER BY l.created_at DESC LIMIT 100"
    )
    .bind(claims.organization_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(logs))
}
