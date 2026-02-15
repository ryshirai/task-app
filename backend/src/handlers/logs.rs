use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use crate::models::*;
use crate::AppState;
use sqlx::{Postgres, QueryBuilder};

fn append_log_filters(
    query_builder: &mut QueryBuilder<Postgres>,
    query: &LogQuery,
    organization_id: i32,
) {
    query_builder
        .push(" WHERE l.organization_id = ")
        .push_bind(organization_id);

    if let Some(user_id) = query.user_id {
        query_builder.push(" AND l.user_id = ").push_bind(user_id);
    }

    if let Some(start_date) = query.start_date {
        query_builder
            .push(" AND l.created_at::date >= ")
            .push_bind(start_date);
    }

    if let Some(end_date) = query.end_date {
        query_builder
            .push(" AND l.created_at::date <= ")
            .push_bind(end_date);
    }

    if let Some(action) = &query.action {
        query_builder
            .push(" AND l.action = ")
            .push_bind(action.clone());
    }

    if let Some(target_type) = &query.target_type {
        query_builder
            .push(" AND l.target_type = ")
            .push_bind(target_type.clone());
    }
}

fn validate_date_range(query: &LogQuery) -> Result<(), (StatusCode, String)> {
    if let (Some(start), Some(end)) = (query.start_date, query.end_date) {
        if start > end {
            return Err((
                StatusCode::BAD_REQUEST,
                "start_date must be before or equal to end_date".to_string(),
            ));
        }
    }
    Ok(())
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn logs_to_csv(logs: &[ActivityLog]) -> String {
    let mut csv = String::from("Date,User,Action,Target Type,Target ID,Details\n");

    for log in logs {
        let date = csv_escape(&log.created_at.to_rfc3339());
        let user = csv_escape(&log.user_name);
        let action = csv_escape(&log.action);
        let target_type = csv_escape(&log.target_type);
        let target_id = csv_escape(
            &log
                .target_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
        );
        let details = csv_escape(log.details.as_deref().unwrap_or(""));

        csv.push_str(&format!(
            "{date},{user},{action},{target_type},{target_id},{details}\n"
        ));
    }

    csv
}

pub async fn get_logs(
    State(state): State<AppState>,
    Query(query): Query<LogQuery>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<PaginatedLogs>, (StatusCode, String)> {
    validate_date_range(&query)?;

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).max(1);
    let offset = (page - 1) * per_page;

    let mut logs_query_builder = QueryBuilder::<Postgres>::new(
        "SELECT l.id, l.organization_id, l.user_id, u.name as user_name, l.action, l.target_type, l.target_id, l.details, l.created_at \
         FROM activity_logs l JOIN users u ON l.user_id = u.id",
    );
    append_log_filters(&mut logs_query_builder, &query, claims.organization_id);
    logs_query_builder
        .push(" ORDER BY l.created_at DESC LIMIT ")
        .push_bind(per_page)
        .push(" OFFSET ")
        .push_bind(offset);

    let logs = logs_query_builder
        .build_query_as::<ActivityLog>()
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut total_query_builder =
        QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM activity_logs l");
    append_log_filters(&mut total_query_builder, &query, claims.organization_id);

    let total = total_query_builder
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

pub async fn export_logs(
    State(state): State<AppState>,
    Query(query): Query<LogQuery>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    validate_date_range(&query)?;

    let mut logs_query_builder = QueryBuilder::<Postgres>::new(
        "SELECT l.id, l.organization_id, l.user_id, u.name as user_name, l.action, l.target_type, l.target_id, l.details, l.created_at \
         FROM activity_logs l JOIN users u ON l.user_id = u.id",
    );
    append_log_filters(&mut logs_query_builder, &query, claims.organization_id);
    logs_query_builder.push(" ORDER BY l.created_at DESC");

    let logs = logs_query_builder
        .build_query_as::<ActivityLog>()
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let csv = logs_to_csv(&logs);

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/csv"));
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"activity_logs.csv\""),
    );

    Ok((headers, csv))
}
