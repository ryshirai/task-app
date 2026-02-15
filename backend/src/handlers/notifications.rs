use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde_json::json;

use crate::AppState;
use crate::models::{Claims, Notification, NotificationQuery, PaginatedNotifications};

pub async fn get_notifications(
    State(state): State<AppState>,
    Query(query): Query<NotificationQuery>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<PaginatedNotifications>, (StatusCode, String)> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let items = sqlx::query_as::<_, Notification>(
        "SELECT id, organization_id, user_id, title, body, category, target_type, target_id, is_read, created_at
         FROM notifications
         WHERE organization_id = $1
           AND user_id = $2
           AND (is_read = FALSE OR created_at >= NOW() - INTERVAL '30 days')
         ORDER BY is_read ASC, created_at DESC
         LIMIT $3 OFFSET $4",
    )
    .bind(claims.organization_id)
    .bind(claims.user_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM notifications
         WHERE organization_id = $1
           AND user_id = $2
           AND (is_read = FALSE OR created_at >= NOW() - INTERVAL '30 days')",
    )
    .bind(claims.organization_id)
    .bind(claims.user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_pages = if total == 0 {
        0
    } else {
        (total + per_page - 1) / per_page
    };

    Ok(Json(PaginatedNotifications {
        items,
        total,
        page,
        total_pages,
    }))
}

pub async fn mark_as_read(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i32>,
) -> Result<Json<Notification>, (StatusCode, String)> {
    let notification = sqlx::query_as::<_, Notification>(
        "UPDATE notifications
         SET is_read = TRUE
         WHERE id = $1 AND organization_id = $2 AND user_id = $3
         RETURNING id, organization_id, user_id, title, body, category, target_type, target_id, is_read, created_at",
    )
    .bind(id)
    .bind(claims.organization_id)
    .bind(claims.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Notification not found".to_string()))?;

    Ok(Json(notification))
}

pub async fn mark_all_as_read(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let result = sqlx::query(
        "UPDATE notifications
         SET is_read = TRUE
         WHERE organization_id = $1 AND user_id = $2 AND is_read = FALSE",
    )
    .bind(claims.organization_id)
    .bind(claims.user_id)
    .execute(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({ "updated": result.rows_affected() })))
}
