use crate::AppState;
use crate::models::*;
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use sqlx::{Pool, Postgres};

#[derive(sqlx::FromRow)]
struct TaskCompletionStats {
    total_completed: i64,
    completed_this_week: i64,
    completed_last_week: i64,
}

async fn fetch_user_analytics(
    pool: &Pool<Postgres>,
    org_id: i32,
    user_id: i32,
) -> Result<AnalyticsResponse, (StatusCode, String)> {
    let user_name = sqlx::query_scalar::<_, String>(
        "SELECT name FROM users WHERE organization_id = $1 AND id = $2",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let task_completion = sqlx::query_as::<_, TaskCompletionStats>(
        "SELECT
            COUNT(*) FILTER (WHERE status = 'done') AS total_completed,
            COUNT(*) FILTER (
                WHERE status = 'done'
                  AND end_at >= date_trunc('week', NOW() AT TIME ZONE 'Asia/Tokyo')
                  AND end_at < date_trunc('week', NOW() AT TIME ZONE 'Asia/Tokyo') + interval '1 week'
            ) AS completed_this_week,
            COUNT(*) FILTER (
                WHERE status = 'done'
                  AND end_at >= date_trunc('week', NOW() AT TIME ZONE 'Asia/Tokyo') - interval '1 week'
                  AND end_at < date_trunc('week', NOW() AT TIME ZONE 'Asia/Tokyo')
            ) AS completed_last_week
         FROM tasks
         WHERE organization_id = $1 AND member_id = $2",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let by_status = sqlx::query_as::<_, StatusCount>(
        "SELECT status, COUNT(*) AS count
         FROM tasks
         WHERE organization_id = $1 AND member_id = $2
         GROUP BY status
         ORDER BY count DESC, status ASC",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_reports = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM daily_reports
         WHERE organization_id = $1 AND user_id = $2",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let heatmap = sqlx::query_as::<_, HeatmapDay>(
        "SELECT
            gs.day::date AS date,
            COALESCE(COUNT(al.id), 0)::BIGINT AS count
         FROM generate_series(
                (CURRENT_TIMESTAMP AT TIME ZONE 'Asia/Tokyo')::date - interval '29 days',
                (CURRENT_TIMESTAMP AT TIME ZONE 'Asia/Tokyo')::date,
                interval '1 day'
         ) AS gs(day)
         LEFT JOIN activity_logs al
           ON al.organization_id = $1
          AND al.user_id = $2
          AND (al.created_at AT TIME ZONE 'Asia/Tokyo')::date = gs.day::date
         GROUP BY gs.day
         ORDER BY gs.day",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(AnalyticsResponse {
        user_name,
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
    })
}

pub async fn get_personal_analytics(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<AnalyticsResponse>, (StatusCode, String)> {
    let analytics =
        fetch_user_analytics(&state.pool, claims.organization_id, claims.user_id).await?;
    Ok(Json(analytics))
}

pub async fn get_user_analytics(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i32>,
) -> Result<Json<AnalyticsResponse>, (StatusCode, String)> {
    if claims.role != "admin" && claims.user_id != id {
        return Err((StatusCode::FORBIDDEN, "Forbidden".to_string()));
    }

    let analytics = fetch_user_analytics(&state.pool, claims.organization_id, id).await?;
    Ok(Json(analytics))
}
