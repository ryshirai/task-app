use crate::AppState;
use crate::models::*;
use axum::{
    Extension, Json,
    extract::State,
    http::StatusCode,
};

#[derive(sqlx::FromRow)]
struct TaskCompletionStats {
    total_completed: i64,
    completed_this_week: i64,
    completed_last_week: i64,
}

pub async fn get_personal_analytics(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<AnalyticsResponse>, (StatusCode, String)> {
    let task_completion = sqlx::query_as::<_, TaskCompletionStats>(
        "SELECT
            COUNT(*) FILTER (WHERE status = 'done') AS total_completed,
            COUNT(*) FILTER (
                WHERE status = 'done'
                  AND end_at >= date_trunc('week', NOW())
                  AND end_at < date_trunc('week', NOW()) + interval '1 week'
            ) AS completed_this_week,
            COUNT(*) FILTER (
                WHERE status = 'done'
                  AND end_at >= date_trunc('week', NOW()) - interval '1 week'
                  AND end_at < date_trunc('week', NOW())
            ) AS completed_last_week
         FROM tasks
         WHERE organization_id = $1 AND member_id = $2",
    )
    .bind(claims.organization_id)
    .bind(claims.user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let by_status = sqlx::query_as::<_, StatusCount>(
        "SELECT status, COUNT(*) AS count
         FROM tasks
         WHERE organization_id = $1 AND member_id = $2
         GROUP BY status
         ORDER BY count DESC, status ASC",
    )
    .bind(claims.organization_id)
    .bind(claims.user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_reports = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM daily_reports
         WHERE organization_id = $1 AND user_id = $2",
    )
    .bind(claims.organization_id)
    .bind(claims.user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let heatmap = sqlx::query_as::<_, HeatmapDay>(
        "SELECT
            gs.day::date AS date,
            COALESCE(COUNT(al.id), 0)::BIGINT AS count
         FROM generate_series(
                CURRENT_DATE - interval '29 days',
                CURRENT_DATE,
                interval '1 day'
         ) AS gs(day)
         LEFT JOIN activity_logs al
           ON al.organization_id = $1
          AND al.user_id = $2
          AND al.created_at::date = gs.day::date
         GROUP BY gs.day
         ORDER BY gs.day",
    )
    .bind(claims.organization_id)
    .bind(claims.user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AnalyticsResponse {
        task_stats: TaskStats {
            total_completed: task_completion.total_completed,
            completed_this_week: task_completion.completed_this_week,
            completed_last_week: task_completion.completed_last_week,
            by_status,
        },
        report_stats: ReportStats {
            total_submitted: total_reports,
        },
        heatmap,
    }))
}
