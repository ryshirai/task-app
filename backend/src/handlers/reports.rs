use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json, Extension,
};
use crate::models::*;
use crate::AppState;
use crate::handlers::log_activity;

pub async fn get_reports(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<DailyReport>>, (StatusCode, String)> {
    let reports = sqlx::query_as::<_, DailyReport>("SELECT * FROM daily_reports WHERE organization_id = $1 ORDER BY report_date DESC")
        .bind(claims.organization_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(reports))
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
    ).await;

    Ok((StatusCode::CREATED, Json(report)))
}

pub async fn update_report(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i32>,
    Json(input): Json<UpdateReportInput>,
) -> Result<Json<DailyReport>, (StatusCode, String)> {
    let report = sqlx::query_as::<_, DailyReport>("SELECT * FROM daily_reports WHERE id = $1 AND organization_id = $2")
        .bind(id)
        .bind(claims.organization_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Report not found".to_string()))?;

    if report.user_id != claims.user_id && claims.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "You can only edit your own reports".to_string()));
    }

    let updated_report = sqlx::query_as::<_, DailyReport>(
        "UPDATE daily_reports SET content = $1 WHERE id = $2 RETURNING *",
    )
    .bind(input.content)
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    log_activity(
        &state.pool,
        claims.organization_id,
        claims.user_id,
        "report_updated",
        "report",
        Some(updated_report.id),
        None,
    ).await;

    Ok(Json(updated_report))
}
