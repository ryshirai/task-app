use crate::AppState;
use crate::WsMessage;
use crate::handlers::{log_activity, notify_user};
use crate::models::*;
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::IntoResponse,
};
use serde_json::json;
use sqlx::{Postgres, QueryBuilder};

async fn user_in_organization(
    state: &AppState,
    organization_id: i32,
    user_id: i32,
) -> Result<bool, (StatusCode, String)> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1 AND organization_id = $2)",
    )
    .bind(user_id)
    .bind(organization_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(exists)
}

fn validate_report_date_range(query: &TaskReportQuery) -> Result<(), (StatusCode, String)> {
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

fn append_task_report_filters(
    query_builder: &mut QueryBuilder<Postgres>,
    query: &TaskReportQuery,
    organization_id: i32,
) {
    query_builder
        .push(" WHERE t.organization_id = ")
        .push_bind(organization_id);

    if let Some(member_id) = query.member_id {
        query_builder
            .push(" AND t.member_id = ")
            .push_bind(member_id);
    }

    if let Some(start_date) = query.start_date {
        query_builder
            .push(" AND t.start_at::date >= ")
            .push_bind(start_date);
    }

    if let Some(end_date) = query.end_date {
        query_builder
            .push(" AND t.end_at::date <= ")
            .push_bind(end_date);
    }

    if let Some(statuses) = &query.statuses {
        let statuses: Vec<String> = statuses
            .split(',')
            .map(str::trim)
            .filter(|status| !status.is_empty())
            .map(str::to_string)
            .collect();

        if statuses.is_empty() {
            return;
        }

        query_builder
            .push(" AND t.status = ANY(")
            .push_bind(statuses)
            .push(")");
    }
}

async fn fetch_task_report_rows(
    state: &AppState,
    organization_id: i32,
    query: &TaskReportQuery,
) -> Result<Vec<TaskWithUser>, (StatusCode, String)> {
    let mut query_builder = QueryBuilder::<Postgres>::new(
        "SELECT t.id, t.organization_id, t.member_id, t.title, t.status, t.progress_rate, t.tags, t.start_at, t.end_at, t.created_at, \
         u.name AS user_name FROM tasks t \
         JOIN users u ON t.member_id = u.id",
    );
    append_task_report_filters(&mut query_builder, query, organization_id);
    query_builder.push(" ORDER BY t.start_at ASC, t.id ASC");

    query_builder
        .build_query_as::<TaskWithUser>()
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn task_report_to_csv(rows: &[TaskWithUser]) -> String {
    let mut csv = String::from("担当者,タスク名,ステータス,進捗率,タグ,開始日時,終了日時\n");
    for row in rows {
        let tags = row
            .task
            .tags
            .as_ref()
            .map(|v| v.join("|"))
            .unwrap_or_default();
        csv.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            csv_escape(&row.user_name),
            csv_escape(&row.task.title),
            csv_escape(&row.task.status),
            row.task.progress_rate,
            csv_escape(&tags),
            csv_escape(&row.task.start_at.to_rfc3339()),
            csv_escape(&row.task.end_at.to_rfc3339()),
        ));
    }
    csv
}

pub async fn create_task(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(input): Json<CreateTaskInput>,
) -> Result<(StatusCode, Json<Task>), (StatusCode, String)> {
    if !user_in_organization(&state, claims.organization_id, input.member_id).await? {
        return Err((StatusCode::BAD_REQUEST, "Invalid member_id".to_string()));
    }

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
    )
    .await;

    if task.member_id != claims.user_id {
        let body = format!("A task was assigned to you: {}", task.title);
        notify_user(
            &state.pool,
            claims.organization_id,
            task.member_id,
            "New task assignment",
            Some(&body),
            "task_assigned",
            Some("task"),
            Some(task.id),
        )
        .await;
    }

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
    let current_task =
        sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = $1 AND organization_id = $2")
            .bind(id)
            .bind(claims.organization_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((StatusCode::NOT_FOUND, "Task not found".to_string()))?;

    if let Some(new_member_id) = input.member_id {
        if !user_in_organization(&state, claims.organization_id, new_member_id).await? {
            return Err((StatusCode::BAD_REQUEST, "Invalid member_id".to_string()));
        }
    }

    let task = sqlx::query_as::<_, Task>(
        "UPDATE tasks SET 
            member_id = COALESCE($1, member_id),
            title = COALESCE($2, title),
            status = COALESCE($3, status),
            progress_rate = COALESCE($4, progress_rate),
            tags = COALESCE($5, tags),
            start_at = COALESCE($6, start_at),
            end_at = COALESCE($7, end_at)
        WHERE id = $8 AND organization_id = $9 RETURNING *",
    )
    .bind(input.member_id)
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
    if current_task.member_id != task.member_id {
        changes.push(
            json!({ "field": "member_id", "old": current_task.member_id, "new": task.member_id }),
        );
    }
    if current_task.status != task.status {
        changes
            .push(json!({ "field": "status", "old": &current_task.status, "new": &task.status }));
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
        changes.push(
            json!({ "field": "start_at", "old": &current_task.start_at, "new": &task.start_at }),
        );
    }
    if current_task.end_at != task.end_at {
        changes
            .push(json!({ "field": "end_at", "old": &current_task.end_at, "new": &task.end_at }));
    }

    log_activity(
        &state.pool,
        claims.organization_id,
        claims.user_id,
        "task_updated",
        "task",
        Some(task.id),
        Some(json!({ "changes": changes }).to_string()),
    )
    .await;

    if current_task.member_id != task.member_id && task.member_id != claims.user_id {
        let body = format!("A task was reassigned to you: {}", task.title);
        notify_user(
            &state.pool,
            claims.organization_id,
            task.member_id,
            "Task reassigned",
            Some(&body),
            "task_reassigned",
            Some("task"),
            Some(task.id),
        )
        .await;
    }

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
    )
    .await;

    let _ = state.tx.send(WsMessage {
        organization_id: claims.organization_id,
        event: "task_deleted".to_string(),
        payload: json!({ "id": id }),
    });

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_task_report(
    State(state): State<AppState>,
    Query(query): Query<TaskReportQuery>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<TaskWithUser>>, (StatusCode, String)> {
    if claims.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Admin access required".to_string()));
    }

    validate_report_date_range(&query)?;
    let rows = fetch_task_report_rows(&state, claims.organization_id, &query).await?;
    Ok(Json(rows))
}

pub async fn export_task_report(
    State(state): State<AppState>,
    Query(query): Query<TaskReportQuery>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if claims.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Admin access required".to_string()));
    }

    validate_report_date_range(&query)?;
    let rows = fetch_task_report_rows(&state, claims.organization_id, &query).await?;
    let csv = task_report_to_csv(&rows);

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/csv"));
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"task_report.csv\""),
    );

    Ok((headers, csv))
}
