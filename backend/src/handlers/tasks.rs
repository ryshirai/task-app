use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json, Extension,
};
use crate::models::*;
use crate::AppState;
use crate::handlers::log_activity;
use crate::WsMessage;
use serde_json::json;

pub async fn create_task(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(input): Json<CreateTaskInput>,
) -> Result<(StatusCode, Json<Task>), (StatusCode, String)> {
    let task = sqlx::query_as::<_, Task>(
        "INSERT INTO tasks (organization_id, member_id, title, tags, start_at, end_at) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
    )
    .bind(claims.organization_id)
    .bind(input.member_id)
    .bind(&input.title)
    .bind(&input.tags)
    .bind(input.start_at)
    .bind(input.end_at)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    log_activity(
        &state.pool,
        claims.organization_id,
        claims.user_id,
        "task_created",
        "task",
        Some(task.id),
        Some(format!("Title: {}", task.title)),
    ).await;

    let _ = state.tx.send(WsMessage {
        organization_id: task.organization_id,
        event: "task_created".to_string(),
        payload: json!(task),
    });

    Ok((StatusCode::CREATED, Json(task)))
}

pub async fn update_task(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i32>,
    Json(input): Json<UpdateTaskInput>,
) -> Result<Json<Task>, (StatusCode, String)> {
    let current_task = sqlx::query_as::<_, Task>(
        "SELECT * FROM tasks WHERE id = $1 AND organization_id = $2",
    )
    .bind(id)
    .bind(claims.organization_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Task not found".to_string()))?;

    let task = sqlx::query_as::<_, Task>(
        "UPDATE tasks SET 
            title = COALESCE($1, title),
            status = COALESCE($2, status),
            progress_rate = COALESCE($3, progress_rate),
            tags = COALESCE($4, tags),
            start_at = COALESCE($5, start_at),
            end_at = COALESCE($6, end_at)
        WHERE id = $7 AND organization_id = $8 RETURNING *",
    )
    .bind(input.title)
    .bind(input.status)
    .bind(input.progress_rate)
    .bind(input.tags)
    .bind(input.start_at)
    .bind(input.end_at)
    .bind(id)
    .bind(claims.organization_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut changes = Vec::new();
    if current_task.title != task.title {
        changes.push(json!({ "field": "title", "old": &current_task.title, "new": &task.title }));
    }
    if current_task.status != task.status {
        changes.push(json!({ "field": "status", "old": &current_task.status, "new": &task.status }));
    }
    if current_task.progress_rate != task.progress_rate {
        changes.push(json!({
            "field": "progress_rate",
            "old": current_task.progress_rate,
            "new": task.progress_rate
        }));
    }
    if current_task.tags != task.tags {
        changes.push(json!({ "field": "tags", "old": &current_task.tags, "new": &task.tags }));
    }
    if current_task.start_at != task.start_at {
        changes.push(json!({ "field": "start_at", "old": &current_task.start_at, "new": &task.start_at }));
    }
    if current_task.end_at != task.end_at {
        changes.push(json!({ "field": "end_at", "old": &current_task.end_at, "new": &task.end_at }));
    }

    log_activity(
        &state.pool,
        claims.organization_id,
        claims.user_id,
        "task_updated",
        "task",
        Some(task.id),
        Some(json!({ "changes": changes }).to_string()),
    ).await;

    let _ = state.tx.send(WsMessage {
        organization_id: task.organization_id,
        event: "task_updated".to_string(),
        payload: json!(task),
    });

    Ok(Json(task))
}

pub async fn delete_task(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i32>,
) -> Result<StatusCode, (StatusCode, String)> {
    sqlx::query("DELETE FROM tasks WHERE id = $1 AND organization_id = $2")
        .bind(id)
        .bind(claims.organization_id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    log_activity(
        &state.pool,
        claims.organization_id,
        claims.user_id,
        "task_deleted",
        "task",
        Some(id),
        None,
    ).await;

    let _ = state.tx.send(WsMessage {
        organization_id: claims.organization_id,
        event: "task_deleted".to_string(),
        payload: json!({ "id": id }),
    });

    Ok(StatusCode::NO_CONTENT)
}
