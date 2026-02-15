use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json, Extension,
};
use crate::models::*;
use crate::AppState;

pub async fn get_logs(
    State(state): State<AppState>,
    Query(query): Query<LogQuery>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<PaginatedLogs>, (StatusCode, String)> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).max(1);
    let offset = (page - 1) * per_page;

    let logs = sqlx::query_as::<_, ActivityLog>(
        "SELECT l.*, u.name as user_name FROM activity_logs l 
         JOIN users u ON l.user_id = u.id 
         WHERE l.organization_id = $1
         ORDER BY l.created_at DESC
         LIMIT $2 OFFSET $3"
    )
    .bind(claims.organization_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM activity_logs WHERE organization_id = $1"
    )
    .bind(claims.organization_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_pages = if total == 0 {
        0
    } else {
        (total + per_page - 1) / per_page
    };

    Ok(Json(PaginatedLogs {
        items: logs,
        total,
        page,
        total_pages,
    }))
}
