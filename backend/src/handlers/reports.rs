use crate::AppState;
use crate::handlers::log_activity;
use crate::models::*;
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde_json::json;

pub async fn get_reports(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<ReportQuery>,
) -> Result<Json<Vec<DailyReport>>, (StatusCode, String)> {
    let mut query = String::from("SELECT * FROM daily_reports WHERE organization_id = $1");
    let mut param_index = 2;

    if params.date.is_some() {
        query.push_str(&format!(" AND report_date = ${}", param_index));
        param_index += 1;
    }
    if params.user_id.is_some() {
        query.push_str(&format!(" AND user_id = ${}", param_index));
    }

    query.push_str(" ORDER BY report_date DESC, created_at DESC");

    let mut q = sqlx::query_as::<_, DailyReport>(&query).bind(claims.organization_id);

    if let Some(date) = params.date {
        q = q.bind(date);
    }
    if let Some(user_id) = params.user_id {
        q = q.bind(user_id);
    }

    let reports = q
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(reports))
}

pub async fn get_report(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i32>,
) -> Result<Json<DailyReport>, (StatusCode, String)> {
    let report = sqlx::query_as::<_, DailyReport>(
        "SELECT * FROM daily_reports WHERE id = $1 AND organization_id = $2",
    )
    .bind(id)
    .bind(claims.organization_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Report not found".to_string()))?;

    Ok(Json(report))
}

pub async fn create_report(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(input): Json<CreateReportInput>,
) -> Result<(StatusCode, Json<DailyReport>), (StatusCode, String)> {
    let user_id = claims.user_id;

    let report = sqlx::query_as::<_, DailyReport>(
        "INSERT INTO daily_reports (organization_id, user_id, report_date, content) VALUES ($1, $2, $3, $4) 
         ON CONFLICT (user_id, report_date) DO UPDATE SET content = $4 RETURNING *",
    )
    .bind(claims.organization_id)
    .bind(user_id)
    .bind(input.report_date)
    .bind(input.content)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    log_activity(
        &state.pool,
        claims.organization_id,
        claims.user_id,
        "report_submitted",
        "report",
        Some(report.id),
        Some(format!("Date: {}", report.report_date)),
    )
    .await;

    Ok((StatusCode::CREATED, Json(report)))
}

pub async fn update_report(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i32>,
    Json(input): Json<UpdateReportInput>,
) -> Result<Json<DailyReport>, (StatusCode, String)> {
    // 権限確認 (自身のレポートか、管理者であること)
    // 完全に1クエリにまとめることも可能だが、ビジネスロジック（Admin権限）の可読性と
    // 安全性のために、まず対象が存在し、かつ権限があるかを確認する
    let report = sqlx::query_as::<_, DailyReport>(
        "SELECT * FROM daily_reports WHERE id = $1 AND organization_id = $2",
    )
    .bind(id)
    .bind(claims.organization_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Report not found".to_string()))?;

    if report.user_id != claims.user_id && claims.role != "admin" {
        return Err((
            StatusCode::FORBIDDEN,
            "You can only edit your own reports".to_string(),
        ));
    }

    let updated_report = sqlx::query_as::<_, DailyReport>(
        "UPDATE daily_reports SET content = $1 WHERE id = $2 AND organization_id = $3 RETURNING *",
    )
    .bind(input.content)
    .bind(id)
    .bind(claims.organization_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut changes = Vec::new();
    if report.content != updated_report.content {
        changes.push(json!({
            "field": "content",
            "old": &report.content,
            "new": &updated_report.content
        }));
    }

    log_activity(
        &state.pool,
        claims.organization_id,
        claims.user_id,
        "report_updated",
        "report",
        Some(updated_report.id),
        Some(json!({ "changes": changes }).to_string()),
    )
    .await;

    Ok(Json(updated_report))
}
