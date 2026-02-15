use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json, Extension,
};
use sqlx::{Postgres, QueryBuilder};
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

    let mut logs_query = QueryBuilder::<Postgres>::new(
        "SELECT l.*, u.name as user_name FROM activity_logs l \
         JOIN users u ON l.user_id = u.id \
         WHERE l.organization_id = "
    );
    logs_query.push_bind(claims.organization_id);

    if let Some(user_id) = query.user_id {
        logs_query.push(" AND l.user_id = ");
        logs_query.push_bind(user_id);
    }

    if let Some(start_date) = query.start_date {
        logs_query.push(" AND l.created_at >= ");
        logs_query.push_bind(start_date.and_hms_opt(0, 0, 0).unwrap().and_utc());
    }

    if let Some(end_date) = query.end_date {
        logs_query.push(" AND l.created_at <= ");
        logs_query.push_bind(end_date.and_hms_opt(23, 59, 59).unwrap().and_utc());
    }

    logs_query.push(" ORDER BY l.created_at DESC LIMIT ");
    logs_query.push_bind(per_page);
    logs_query.push(" OFFSET ");
    logs_query.push_bind(offset);

    let logs = logs_query
        .build_query_as::<ActivityLog>()
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut count_query =
        QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM activity_logs l WHERE l.organization_id = ");
    count_query.push_bind(claims.organization_id);

    if let Some(user_id) = query.user_id {
        count_query.push(" AND l.user_id = ");
        count_query.push_bind(user_id);
    }

    if let Some(start_date) = query.start_date {
        count_query.push(" AND l.created_at >= ");
        count_query.push_bind(start_date.and_hms_opt(0, 0, 0).unwrap().and_utc());
    }

    if let Some(end_date) = query.end_date {
        count_query.push(" AND l.created_at <= ");
        count_query.push_bind(end_date.and_hms_opt(23, 59, 59).unwrap().and_utc());
    }

    let total = count_query
        .build_query_scalar::<i64>()
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
