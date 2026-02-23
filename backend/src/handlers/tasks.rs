use crate::AppState;
use crate::models::{
    AddTimeLogInput, Claims, CreateTaskInput, D1Param, D1Row, GetTasksQuery, ModelError, Task,
    TaskReportQuery, TaskReportRow, TaskTimeLog, UpdateTaskInput, UpdateTimeLogInput, d1_execute,
    d1_query_all, d1_query_one,
};
use chrono::{DateTime, FixedOffset};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use worker::{Request, Response, Result as WorkerResult, RouteContext};

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

#[derive(Debug)]
struct ApiError {
    status: u16,
    message: String,
}

impl ApiError {
    fn new(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self::new(500, message)
    }

    fn into_response(self) -> WorkerResult<Response> {
        Response::from_json(&ErrorBody {
            error: self.message,
        })
        .map(|response| response.with_status(self.status))
    }
}

impl From<ModelError> for ApiError {
    fn from(value: ModelError) -> Self {
        Self::internal(value.to_string())
    }
}

impl From<worker::Error> for ApiError {
    fn from(value: worker::Error) -> Self {
        Self::internal(value.to_string())
    }
}

#[derive(Clone, Debug)]
struct CountRow {
    count: i64,
}

impl crate::models::FromD1Row for CountRow {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError> {
        let count = row
            .get("count")
            .and_then(Value::as_i64)
            .ok_or(ModelError::MissingField("count"))?;
        Ok(Self { count })
    }
}

#[derive(Clone, Debug)]
struct IdRow {
    id: i64,
}

impl crate::models::FromD1Row for IdRow {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError> {
        let id = row
            .get("id")
            .and_then(Value::as_i64)
            .ok_or(ModelError::MissingField("id"))?;
        Ok(Self { id })
    }
}

#[derive(Clone, Debug)]
struct RoleRow {
    role: String,
}

impl crate::models::FromD1Row for RoleRow {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError> {
        let role = row
            .get("role")
            .and_then(Value::as_str)
            .ok_or(ModelError::MissingField("role"))?
            .to_string();
        Ok(Self { role })
    }
}

#[derive(Clone, Debug)]
struct ReportFlatRow {
    id: i64,
    organization_id: i64,
    member_id: i64,
    title: String,
    description: Option<String>,
    status: String,
    progress_rate: i64,
    tags: Option<Vec<String>>,
    created_at: String,
    updated_at: Option<String>,
    total_duration_minutes: i64,
    user_name: String,
    start_at: Option<String>,
    end_at: Option<String>,
}

impl crate::models::FromD1Row for ReportFlatRow {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError> {
        let tags = match row.get("tags") {
            None | Some(Value::Null) => None,
            Some(Value::String(raw)) if raw.trim().is_empty() => Some(vec![]),
            Some(Value::String(raw)) => {
                Some(raw.split(',').map(|v| v.trim().to_string()).collect())
            }
            Some(Value::Array(items)) => {
                let mut parsed = Vec::with_capacity(items.len());
                for item in items {
                    let text = item.as_str().ok_or(ModelError::InvalidType {
                        field: "tags",
                        expected: "array<string>",
                    })?;
                    parsed.push(text.to_string());
                }
                Some(parsed)
            }
            Some(_) => {
                return Err(ModelError::InvalidType {
                    field: "tags",
                    expected: "string|array<string>|null",
                });
            }
        };

        let required_i64 = |field: &'static str| {
            row.get(field)
                .and_then(Value::as_i64)
                .ok_or(ModelError::MissingField(field))
        };
        let required_text = |field: &'static str| {
            row.get(field)
                .and_then(Value::as_str)
                .map(|v| v.to_string())
                .ok_or(ModelError::MissingField(field))
        };
        let optional_text = |field: &'static str| match row.get(field) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(v)) => Ok(Some(v.clone())),
            _ => Err(ModelError::InvalidType {
                field,
                expected: "text|null",
            }),
        };

        Ok(Self {
            id: required_i64("id")?,
            organization_id: required_i64("organization_id")?,
            member_id: required_i64("member_id")?,
            title: required_text("title")?,
            description: optional_text("description")?,
            status: required_text("status")?,
            progress_rate: required_i64("progress_rate")?,
            tags,
            created_at: required_text("created_at")?,
            updated_at: optional_text("updated_at")?,
            total_duration_minutes: row
                .get("total_duration_minutes")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            user_name: required_text("user_name")?,
            start_at: optional_text("start_at")?,
            end_at: optional_text("end_at")?,
        })
    }
}

fn json_with_status<T: Serialize>(value: &T, status: u16) -> Result<Response, ApiError> {
    Response::from_json(value)
        .map(|response| response.with_status(status))
        .map_err(ApiError::from)
}

fn query_pairs(req: &Request) -> Result<HashMap<String, String>, ApiError> {
    let url = req
        .url()
        .map_err(|e| ApiError::new(400, format!("invalid url: {e}")))?;
    let mut pairs = HashMap::new();
    if let Some(query) = url.query() {
        for pair in query.split('&') {
            if pair.is_empty() {
                continue;
            }
            if let Some((k, v)) = pair.split_once('=') {
                pairs.insert(k.to_string(), v.to_string());
            } else {
                pairs.insert(pair.to_string(), String::new());
            }
        }
    }
    Ok(pairs)
}

fn parse_i64_opt(value: Option<&String>, field: &'static str) -> Result<Option<i64>, ApiError> {
    match value {
        None => Ok(None),
        Some(v) if v.trim().is_empty() => Ok(None),
        Some(v) => v
            .parse::<i64>()
            .map(Some)
            .map_err(|_| ApiError::new(400, format!("invalid {field}"))),
    }
}

fn parse_get_tasks_query(req: &Request) -> Result<GetTasksQuery, ApiError> {
    let pairs = query_pairs(req)?;
    Ok(GetTasksQuery {
        member_id: parse_i64_opt(pairs.get("member_id"), "member_id")?,
        group_id: parse_i64_opt(pairs.get("group_id"), "group_id")?,
        q: pairs.get("q").cloned(),
        date: pairs.get("date").cloned(),
        status: pairs.get("status").cloned(),
    })
}

fn parse_task_report_query(req: &Request) -> Result<TaskReportQuery, ApiError> {
    let pairs = query_pairs(req)?;
    Ok(TaskReportQuery {
        member_id: parse_i64_opt(pairs.get("member_id"), "member_id")?,
        start_date: pairs.get("start_date").cloned(),
        end_date: pairs.get("end_date").cloned(),
        statuses: pairs.get("statuses").cloned(),
    })
}

fn extract_bearer_token(req: &Request) -> Option<String> {
    let header_token = req
        .headers()
        .get("Authorization")
        .ok()
        .flatten()
        .and_then(|v| v.strip_prefix("Bearer ").map(|s| s.to_string()));

    if header_token.is_some() {
        return header_token;
    }

    req.url().ok().and_then(|url| {
        url.query().and_then(|query| {
            query
                .split('&')
                .filter_map(|pair| pair.split_once('='))
                .find_map(|(k, v)| (k == "token" && !v.is_empty()).then_some(v.to_string()))
        })
    })
}

async fn extract_claims(req: &Request, ctx: &RouteContext<AppState>) -> Result<Claims, ApiError> {
    let token = extract_bearer_token(req)
        .ok_or_else(|| ApiError::new(401, "Missing authorization token"))?;

    let token_data = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(ctx.data.jwt_secret.as_ref()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|_| ApiError::new(401, "Invalid token"))?;

    let mut claims = token_data.claims;

    let latest_role = d1_query_one::<RoleRow>(
        &ctx.data.db,
        "SELECT role FROM users WHERE id = ?1 AND organization_id = ?2 LIMIT 1",
        &[
            D1Param::Integer(claims.user_id),
            D1Param::Integer(claims.organization_id),
        ],
    )
    .await?
    .ok_or_else(|| ApiError::new(401, "Unauthorized"))?;

    claims.role = latest_role.role;
    Ok(claims)
}

async fn user_in_organization(
    state: &AppState,
    organization_id: i64,
    user_id: i64,
) -> Result<bool, ApiError> {
    let row = d1_query_one::<CountRow>(
        &state.db,
        "SELECT COUNT(*) AS count FROM users WHERE id = ?1 AND organization_id = ?2",
        &[D1Param::Integer(user_id), D1Param::Integer(organization_id)],
    )
    .await?
    .ok_or_else(|| ApiError::internal("failed to check user organization"))?;

    Ok(row.count > 0)
}

fn validate_report_date_range(query: &TaskReportQuery) -> Result<(), ApiError> {
    if let (Some(start), Some(end)) = (&query.start_date, &query.end_date)
        && start > end
    {
        return Err(ApiError::new(
            400,
            "start_date must be before or equal to end_date",
        ));
    }
    Ok(())
}

fn parse_iso_datetime(input: &str, field: &'static str) -> Result<DateTime<FixedOffset>, ApiError> {
    DateTime::parse_from_rfc3339(input)
        .map_err(|_| ApiError::new(400, format!("{field} must be RFC3339 datetime")))
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn task_report_to_csv(rows: &[TaskReportRow]) -> String {
    let mut csv = String::from(
        "担当者,タスク名,ステータス,進捗率,タグ,開始日時,終了日時,Total Duration (Hours)\n",
    );

    for row in rows {
        let tags = row
            .task
            .tags
            .as_ref()
            .map(|v| v.join("|"))
            .unwrap_or_default();

        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            csv_escape(&row.user_name),
            csv_escape(&row.task.title),
            csv_escape(&row.task.status),
            row.task.progress_rate,
            csv_escape(&tags),
            csv_escape(row.start_at.as_deref().unwrap_or("")),
            csv_escape(row.end_at.as_deref().unwrap_or("")),
            format!("{:.2}", row.total_duration_minutes as f64 / 60.0),
        ));
    }

    csv
}

async fn log_activity_d1(
    state: &AppState,
    organization_id: i64,
    user_id: i64,
    action: &str,
    target_type: &str,
    target_id: Option<i64>,
    details: Option<String>,
) {
    let _ = d1_execute(
        &state.db,
        "INSERT INTO activity_logs (organization_id, user_id, action, target_type, target_id, details)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        &[
            D1Param::Integer(organization_id),
            D1Param::Integer(user_id),
            D1Param::Text(action.to_string()),
            D1Param::Text(target_type.to_string()),
            target_id.map(D1Param::Integer).unwrap_or(D1Param::Null),
            details.map(D1Param::Text).unwrap_or(D1Param::Null),
        ],
    )
    .await;
}

async fn notify_user_d1(
    state: &AppState,
    organization_id: i64,
    user_id: i64,
    title: &str,
    body: Option<&str>,
    category: &str,
    target_type: Option<&str>,
    target_id: Option<i64>,
) {
    let _ = d1_execute(
        &state.db,
        "INSERT INTO notifications (organization_id, user_id, title, body, category, target_type, target_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        &[
            D1Param::Integer(organization_id),
            D1Param::Integer(user_id),
            D1Param::Text(title.to_string()),
            body.map(|v| D1Param::Text(v.to_string()))
                .unwrap_or(D1Param::Null),
            D1Param::Text(category.to_string()),
            target_type
                .map(|v| D1Param::Text(v.to_string()))
                .unwrap_or(D1Param::Null),
            target_id.map(D1Param::Integer).unwrap_or(D1Param::Null),
        ],
    )
    .await;
}

fn task_select_sql() -> &'static str {
    "SELECT t.id, t.organization_id, t.member_id, t.title, t.description, t.status, t.progress_rate,
            NULLIF(GROUP_CONCAT(DISTINCT tg.name), '') AS tags,
            t.created_at, t.updated_at,
            COALESCE(SUM(l.duration_minutes), 0) AS total_duration_minutes
     FROM tasks t
     LEFT JOIN task_tags tt ON t.id = tt.task_id
     LEFT JOIN tags tg ON tt.tag_id = tg.id
     LEFT JOIN task_time_logs l ON l.task_id = t.id AND l.organization_id = t.organization_id"
}

async fn fetch_task_by_id(
    state: &AppState,
    organization_id: i64,
    task_id: i64,
) -> Result<Option<Task>, ApiError> {
    d1_query_one::<Task>(
        &state.db,
        &format!(
            "{} WHERE t.id = ?1 AND t.organization_id = ?2 GROUP BY t.id",
            task_select_sql()
        ),
        &[D1Param::Integer(task_id), D1Param::Integer(organization_id)],
    )
    .await
    .map_err(ApiError::from)
}

async fn upsert_tag_and_link(
    state: &AppState,
    organization_id: i64,
    task_id: i64,
    tag_name: &str,
) -> Result<(), ApiError> {
    d1_execute(
        &state.db,
        "INSERT INTO tags (organization_id, name)
         VALUES (?1, ?2)
         ON CONFLICT (organization_id, name) DO UPDATE SET name = excluded.name",
        &[
            D1Param::Integer(organization_id),
            D1Param::Text(tag_name.to_string()),
        ],
    )
    .await?;

    let tag = d1_query_one::<IdRow>(
        &state.db,
        "SELECT id FROM tags WHERE organization_id = ?1 AND name = ?2 LIMIT 1",
        &[
            D1Param::Integer(organization_id),
            D1Param::Text(tag_name.to_string()),
        ],
    )
    .await?
    .ok_or_else(|| ApiError::internal("failed to resolve tag id"))?;

    d1_execute(
        &state.db,
        "INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?1, ?2)",
        &[D1Param::Integer(task_id), D1Param::Integer(tag.id)],
    )
    .await?;

    Ok(())
}

async fn fetch_time_log_with_task(
    state: &AppState,
    organization_id: i64,
    time_log_id: i64,
) -> Result<TaskTimeLog, ApiError> {
    d1_query_one::<TaskTimeLog>(
        &state.db,
        "SELECT l.id, l.organization_id, l.user_id, l.task_id, l.start_at, l.end_at,
                l.duration_minutes,
                l.created_at,
                t.title AS task_title,
                t.description AS task_description,
                t.status AS task_status,
                t.progress_rate AS task_progress_rate,
                NULLIF(GROUP_CONCAT(DISTINCT tg.name), '') AS task_tags,
                COALESCE(sums.total, 0) AS total_duration_minutes
         FROM task_time_logs l
         JOIN tasks t ON t.id = l.task_id AND t.organization_id = l.organization_id
         LEFT JOIN task_tags tt ON t.id = tt.task_id
         LEFT JOIN tags tg ON tt.tag_id = tg.id
         LEFT JOIN (
             SELECT task_id, SUM(duration_minutes) AS total
             FROM task_time_logs
             WHERE organization_id = ?2
             GROUP BY task_id
         ) sums ON sums.task_id = l.task_id
         WHERE l.id = ?1 AND l.organization_id = ?2
         GROUP BY l.id",
        &[
            D1Param::Integer(time_log_id),
            D1Param::Integer(organization_id),
        ],
    )
    .await?
    .ok_or_else(|| ApiError::new(404, "Time log not found"))
}

fn split_csv_values(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub async fn add_time_log(mut req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let result = async {
        let claims = extract_claims(&req, &ctx).await?;
        let input: AddTimeLogInput = req
            .json()
            .await
            .map_err(|e| ApiError::new(400, e.to_string()))?;

        let start_at = parse_iso_datetime(&input.start_at, "start_at")?;
        let end_at = parse_iso_datetime(&input.end_at, "end_at")?;
        if end_at <= start_at {
            return Err(ApiError::new(400, "end_at must be after start_at"));
        }

        if !user_in_organization(&ctx.data, claims.organization_id, input.user_id).await? {
            return Err(ApiError::new(400, "Invalid user_id"));
        }

        let task_id = if let Some(task_id) = input.task_id {
            let task = fetch_task_by_id(&ctx.data, claims.organization_id, task_id)
                .await?
                .ok_or_else(|| ApiError::new(404, "Task not found"))?;

            if task.member_id != input.user_id {
                return Err(ApiError::new(
                    400,
                    "Selected task does not belong to user_id",
                ));
            }
            task.id
        } else {
            let title = input
                .title
                .as_ref()
                .map(|v| v.trim())
                .filter(|v| !v.is_empty())
                .ok_or_else(|| ApiError::new(400, "title is required"))?
                .to_string();

            let existing = d1_query_one::<Task>(
                &ctx.data.db,
                &format!(
                    "{} WHERE t.organization_id = ?1 AND t.member_id = ?2 AND t.title = ?3 AND t.status != 'done'
                     GROUP BY t.id
                     ORDER BY t.created_at DESC
                     LIMIT 1",
                    task_select_sql()
                ),
                &[
                    D1Param::Integer(claims.organization_id),
                    D1Param::Integer(input.user_id),
                    D1Param::Text(title.clone()),
                ],
            )
            .await?;

            if let Some(task) = existing {
                task.id
            } else {
                d1_execute(
                    &ctx.data.db,
                    "INSERT INTO tasks (organization_id, member_id, title, description)
                     VALUES (?1, ?2, ?3, ?4)",
                    &[
                        D1Param::Integer(claims.organization_id),
                        D1Param::Integer(input.user_id),
                        D1Param::Text(title.clone()),
                        input
                            .description
                            .clone()
                            .map(D1Param::Text)
                            .unwrap_or(D1Param::Null),
                    ],
                )
                .await?;

                let created = d1_query_one::<IdRow>(
                    &ctx.data.db,
                    "SELECT id FROM tasks
                     WHERE organization_id = ?1 AND member_id = ?2 AND title = ?3
                     ORDER BY id DESC LIMIT 1",
                    &[
                        D1Param::Integer(claims.organization_id),
                        D1Param::Integer(input.user_id),
                        D1Param::Text(title),
                    ],
                )
                .await?
                .ok_or_else(|| ApiError::internal("failed to resolve created task id"))?;

                if let Some(tags) = &input.tags {
                    for tag_name in tags {
                        let normalized = tag_name.trim();
                        if normalized.is_empty() {
                            continue;
                        }
                        upsert_tag_and_link(&ctx.data, claims.organization_id, created.id, normalized)
                            .await?;
                    }
                }

                created.id
            }
        };

        d1_execute(
            &ctx.data.db,
            "INSERT INTO task_time_logs (organization_id, user_id, task_id, start_at, end_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            &[
                D1Param::Integer(claims.organization_id),
                D1Param::Integer(input.user_id),
                D1Param::Integer(task_id),
                D1Param::Text(input.start_at.clone()),
                D1Param::Text(input.end_at.clone()),
            ],
        )
        .await?;

        let inserted_log = d1_query_one::<IdRow>(
            &ctx.data.db,
            "SELECT id FROM task_time_logs
             WHERE organization_id = ?1 AND user_id = ?2 AND task_id = ?3 AND start_at = ?4 AND end_at = ?5
             ORDER BY id DESC LIMIT 1",
            &[
                D1Param::Integer(claims.organization_id),
                D1Param::Integer(input.user_id),
                D1Param::Integer(task_id),
                D1Param::Text(input.start_at),
                D1Param::Text(input.end_at),
            ],
        )
        .await?
        .ok_or_else(|| ApiError::internal("failed to resolve created time log id"))?;

        let time_log = fetch_time_log_with_task(&ctx.data, claims.organization_id, inserted_log.id).await?;

        log_activity_d1(
            &ctx.data,
            claims.organization_id,
            claims.user_id,
            "time_log_added",
            "task_time_log",
            Some(time_log.id),
            Some(format!("task_id={}, user_id={}", time_log.task_id, time_log.user_id)),
        )
        .await;

        json_with_status(&time_log, 201)
    }
    .await;

    result.or_else(|e| e.into_response())
}

pub async fn update_time_log(
    mut req: Request,
    ctx: RouteContext<AppState>,
) -> WorkerResult<Response> {
    let result = async {
        let claims = extract_claims(&req, &ctx).await?;
        let id = ctx
            .param("id")
            .and_then(|v| v.parse::<i64>().ok())
            .ok_or_else(|| ApiError::new(400, "invalid id"))?;

        let input: UpdateTimeLogInput = req
            .json()
            .await
            .map_err(|e| ApiError::new(400, e.to_string()))?;

        let current_log = d1_query_one::<TaskTimeLog>(
            &ctx.data.db,
            "SELECT id, organization_id, user_id, task_id, start_at, end_at, duration_minutes, created_at,
                    NULL AS task_title, NULL AS task_description, NULL AS task_status,
                    NULL AS task_progress_rate, NULL AS task_tags, 0 AS total_duration_minutes
             FROM task_time_logs
             WHERE id = ?1 AND organization_id = ?2
             LIMIT 1",
            &[D1Param::Integer(id), D1Param::Integer(claims.organization_id)],
        )
        .await?
        .ok_or_else(|| ApiError::new(404, "Time log not found"))?;

        let next_start = input.start_at.clone().unwrap_or(current_log.start_at);
        let next_end = input.end_at.clone().unwrap_or(current_log.end_at);
        let start = parse_iso_datetime(&next_start, "start_at")?;
        let end = parse_iso_datetime(&next_end, "end_at")?;
        if end <= start {
            return Err(ApiError::new(400, "end_at must be after start_at"));
        }

        d1_execute(
            &ctx.data.db,
            "UPDATE task_time_logs
             SET start_at = COALESCE(?1, start_at),
                 end_at = COALESCE(?2, end_at)
             WHERE id = ?3 AND organization_id = ?4",
            &[
                input.start_at.map(D1Param::Text).unwrap_or(D1Param::Null),
                input.end_at.map(D1Param::Text).unwrap_or(D1Param::Null),
                D1Param::Integer(id),
                D1Param::Integer(claims.organization_id),
            ],
        )
        .await?;

        let updated = fetch_time_log_with_task(&ctx.data, claims.organization_id, id).await?;

        log_activity_d1(
            &ctx.data,
            claims.organization_id,
            claims.user_id,
            "time_log_updated",
            "task_time_log",
            Some(updated.id),
            Some(
                json!({
                    "start_at": updated.start_at,
                    "end_at": updated.end_at,
                    "duration_minutes": updated.duration_minutes,
                })
                .to_string(),
            ),
        )
        .await;

        json_with_status(&updated, 200)
    }
    .await;

    result.or_else(|e| e.into_response())
}

pub async fn delete_time_log(req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let result = async {
        let claims = extract_claims(&req, &ctx).await?;
        let id = ctx
            .param("id")
            .and_then(|v| v.parse::<i64>().ok())
            .ok_or_else(|| ApiError::new(400, "invalid id"))?;

        let exists = d1_query_one::<IdRow>(
            &ctx.data.db,
            "SELECT id FROM task_time_logs WHERE id = ?1 AND organization_id = ?2 LIMIT 1",
            &[
                D1Param::Integer(id),
                D1Param::Integer(claims.organization_id),
            ],
        )
        .await?;
        if exists.is_none() {
            return Err(ApiError::new(404, "Time log not found"));
        }

        d1_execute(
            &ctx.data.db,
            "DELETE FROM task_time_logs WHERE id = ?1 AND organization_id = ?2",
            &[
                D1Param::Integer(id),
                D1Param::Integer(claims.organization_id),
            ],
        )
        .await?;

        log_activity_d1(
            &ctx.data,
            claims.organization_id,
            claims.user_id,
            "time_log_deleted",
            "task_time_log",
            Some(id),
            None,
        )
        .await;

        Ok(Response::empty()?.with_status(204))
    }
    .await;

    result.or_else(|e| e.into_response())
}

pub async fn get_tasks(req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let result = async {
        let claims = extract_claims(&req, &ctx).await?;
        let query = parse_get_tasks_query(&req)?;

        let mut sql = String::from(
            "SELECT t.id, t.organization_id, t.member_id, t.title, t.description, t.status, t.progress_rate,
                    NULLIF(GROUP_CONCAT(DISTINCT tg.name), '') AS tags,
                    t.created_at, t.updated_at,
                    COALESCE(SUM(l.duration_minutes), 0) AS total_duration_minutes
             FROM tasks t
             LEFT JOIN task_time_logs l ON l.task_id = t.id AND l.organization_id = t.organization_id
             LEFT JOIN task_tags tt ON t.id = tt.task_id
             LEFT JOIN tags tg ON tt.tag_id = tg.id
             WHERE t.organization_id = ?",
        );
        let mut params = vec![D1Param::Integer(claims.organization_id)];

        if let Some(member_id) = query.member_id {
            sql.push_str(" AND t.member_id = ?");
            params.push(D1Param::Integer(member_id));
        }

        if let Some(group_id) = query.group_id {
            sql.push_str(
                " AND EXISTS (
                    SELECT 1
                    FROM display_groups dg
                    JOIN display_group_members dgm ON dgm.group_id = dg.id
                    WHERE dg.id = ?
                      AND dg.organization_id = ?
                      AND dg.user_id = ?
                      AND dgm.member_id = t.member_id
                )",
            );
            params.push(D1Param::Integer(group_id));
            params.push(D1Param::Integer(claims.organization_id));
            params.push(D1Param::Integer(claims.user_id));
        }

        if let Some(q) = query.q.map(|v| v.trim().to_string()).filter(|v| !v.is_empty()) {
            let like_pattern = format!("%{q}%");
            sql.push_str(
                " AND (
                    LOWER(t.title) LIKE LOWER(?)
                    OR EXISTS (
                        SELECT 1
                        FROM task_tags tt_q
                        JOIN tags tg_q ON tg_q.id = tt_q.tag_id
                        WHERE tt_q.task_id = t.id
                          AND tg_q.organization_id = ?
                          AND LOWER(tg_q.name) LIKE LOWER(?)
                    )
                    OR EXISTS (
                        SELECT 1
                        FROM users u_q
                        WHERE u_q.id = t.member_id
                          AND u_q.organization_id = ?
                          AND LOWER(COALESCE(u_q.username, '')) LIKE LOWER(?)
                    )
                )",
            );
            params.push(D1Param::Text(like_pattern.clone()));
            params.push(D1Param::Integer(claims.organization_id));
            params.push(D1Param::Text(like_pattern.clone()));
            params.push(D1Param::Integer(claims.organization_id));
            params.push(D1Param::Text(like_pattern));
        }

        if let Some(date) = query.date {
            sql.push_str(
                " AND EXISTS (
                    SELECT 1
                    FROM task_time_logs l_filter
                    WHERE l_filter.task_id = t.id
                      AND l_filter.organization_id = t.organization_id
                      AND date(datetime(l_filter.start_at, '+9 hours')) <= ?
                      AND date(datetime(l_filter.end_at, '+9 hours')) >= ?
                )",
            );
            params.push(D1Param::Text(date.clone()));
            params.push(D1Param::Text(date));
        }

        if let Some(status) = query.status {
            let statuses = split_csv_values(&status);
            if !statuses.is_empty() {
                let placeholders = vec!["?"; statuses.len()].join(", ");
                sql.push_str(&format!(" AND t.status IN ({placeholders})"));
                for v in statuses {
                    params.push(D1Param::Text(v));
                }
            }
        }

        sql.push_str(" GROUP BY t.id ORDER BY t.created_at DESC");

        let tasks = d1_query_all::<Task>(&ctx.data.db, &sql, &params).await?;
        json_with_status(&tasks, 200)
    }
    .await;

    result.or_else(|e| e.into_response())
}

pub async fn create_task(mut req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let result = async {
        let claims = extract_claims(&req, &ctx).await?;
        let input: CreateTaskInput = req
            .json()
            .await
            .map_err(|e| ApiError::new(400, e.to_string()))?;

        if !user_in_organization(&ctx.data, claims.organization_id, input.member_id).await? {
            return Err(ApiError::new(400, "Invalid member_id"));
        }

        d1_execute(
            &ctx.data.db,
            "INSERT INTO tasks (organization_id, member_id, title, description)
             VALUES (?1, ?2, ?3, ?4)",
            &[
                D1Param::Integer(claims.organization_id),
                D1Param::Integer(input.member_id),
                D1Param::Text(input.title.clone()),
                input
                    .description
                    .clone()
                    .map(D1Param::Text)
                    .unwrap_or(D1Param::Null),
            ],
        )
        .await?;

        let created = d1_query_one::<IdRow>(
            &ctx.data.db,
            "SELECT id FROM tasks
             WHERE organization_id = ?1 AND member_id = ?2 AND title = ?3
             ORDER BY id DESC LIMIT 1",
            &[
                D1Param::Integer(claims.organization_id),
                D1Param::Integer(input.member_id),
                D1Param::Text(input.title.clone()),
            ],
        )
        .await?
        .ok_or_else(|| ApiError::internal("failed to resolve created task id"))?;

        if let Some(tags) = &input.tags {
            for tag_name in tags {
                let normalized = tag_name.trim();
                if normalized.is_empty() {
                    continue;
                }
                upsert_tag_and_link(&ctx.data, claims.organization_id, created.id, normalized)
                    .await?;
            }
        }

        let task = fetch_task_by_id(&ctx.data, claims.organization_id, created.id)
            .await?
            .ok_or_else(|| ApiError::internal("failed to load created task"))?;

        log_activity_d1(
            &ctx.data,
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
            notify_user_d1(
                &ctx.data,
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

        json_with_status(&task, 201)
    }
    .await;

    result.or_else(|e| e.into_response())
}

pub async fn update_task(mut req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let result = async {
        let claims = extract_claims(&req, &ctx).await?;
        let id = ctx
            .param("id")
            .and_then(|v| v.parse::<i64>().ok())
            .ok_or_else(|| ApiError::new(400, "invalid id"))?;
        let input: UpdateTaskInput = req
            .json()
            .await
            .map_err(|e| ApiError::new(400, e.to_string()))?;

        let current_task = fetch_task_by_id(&ctx.data, claims.organization_id, id)
            .await?
            .ok_or_else(|| ApiError::new(404, "Task not found"))?;

        if let Some(new_member_id) = input.member_id
            && !user_in_organization(&ctx.data, claims.organization_id, new_member_id).await?
        {
            return Err(ApiError::new(400, "Invalid member_id"));
        }

        d1_execute(
            &ctx.data.db,
            "UPDATE tasks
             SET member_id = COALESCE(?1, member_id),
                 title = COALESCE(?2, title),
                 description = COALESCE(?3, description),
                 status = COALESCE(?4, status),
                 progress_rate = COALESCE(?5, progress_rate),
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?6 AND organization_id = ?7",
            &[
                input.member_id.map(D1Param::Integer).unwrap_or(D1Param::Null),
                input
                    .title
                    .clone()
                    .map(D1Param::Text)
                    .unwrap_or(D1Param::Null),
                input
                    .description
                    .clone()
                    .map(D1Param::Text)
                    .unwrap_or(D1Param::Null),
                input
                    .status
                    .clone()
                    .map(D1Param::Text)
                    .unwrap_or(D1Param::Null),
                input
                    .progress_rate
                    .map(D1Param::Integer)
                    .unwrap_or(D1Param::Null),
                D1Param::Integer(id),
                D1Param::Integer(claims.organization_id),
            ],
        )
        .await?;

        if let Some(tags) = &input.tags {
            d1_execute(
                &ctx.data.db,
                "DELETE FROM task_tags WHERE task_id = ?1",
                &[D1Param::Integer(id)],
            )
            .await?;

            for tag_name in tags {
                let normalized = tag_name.trim();
                if normalized.is_empty() {
                    continue;
                }
                upsert_tag_and_link(&ctx.data, claims.organization_id, id, normalized).await?;
            }
        }

        let task = fetch_task_by_id(&ctx.data, claims.organization_id, id)
            .await?
            .ok_or_else(|| ApiError::new(404, "Task not found"))?;

        let mut changes = Vec::new();
        if current_task.title != task.title {
            changes.push(json!({ "field": "title", "old": current_task.title, "new": task.title }));
        }
        if current_task.status != task.status {
            changes.push(json!({ "field": "status", "old": current_task.status, "new": task.status }));
        }
        if current_task.progress_rate != task.progress_rate {
            changes.push(json!({ "field": "progress_rate", "old": current_task.progress_rate, "new": task.progress_rate }));
        }

        log_activity_d1(
            &ctx.data,
            claims.organization_id,
            claims.user_id,
            "task_updated",
            "task",
            Some(task.id),
            Some(json!({ "changes": changes }).to_string()),
        )
        .await;

        json_with_status(&task, 200)
    }
    .await;

    result.or_else(|e| e.into_response())
}

pub async fn delete_task(req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let result = async {
        let claims = extract_claims(&req, &ctx).await?;
        let id = ctx
            .param("id")
            .and_then(|v| v.parse::<i64>().ok())
            .ok_or_else(|| ApiError::new(400, "invalid id"))?;

        let exists = fetch_task_by_id(&ctx.data, claims.organization_id, id).await?;
        if exists.is_none() {
            return Err(ApiError::new(404, "Task not found"));
        }

        d1_execute(
            &ctx.data.db,
            "DELETE FROM tasks WHERE id = ?1 AND organization_id = ?2",
            &[
                D1Param::Integer(id),
                D1Param::Integer(claims.organization_id),
            ],
        )
        .await?;

        log_activity_d1(
            &ctx.data,
            claims.organization_id,
            claims.user_id,
            "task_deleted",
            "task",
            Some(id),
            None,
        )
        .await;

        Ok(Response::empty()?.with_status(204))
    }
    .await;

    result.or_else(|e| e.into_response())
}

async fn fetch_task_report_rows(
    state: &AppState,
    organization_id: i64,
    query: &TaskReportQuery,
) -> Result<Vec<TaskReportRow>, ApiError> {
    let mut sql = String::from(
        "SELECT t.id, t.organization_id, t.member_id, t.title, t.description, t.status, t.progress_rate,
                NULLIF(GROUP_CONCAT(DISTINCT tg.name), '') AS tags,
                t.created_at, t.updated_at,
                COALESCE(SUM(l.duration_minutes), 0) AS total_duration_minutes,
                u.name AS user_name,
                MIN(l.start_at) AS start_at,
                MAX(l.end_at) AS end_at
         FROM tasks t
         JOIN users u ON t.member_id = u.id
         LEFT JOIN task_tags tt ON t.id = tt.task_id
         LEFT JOIN tags tg ON tt.tag_id = tg.id
         LEFT JOIN task_time_logs l ON l.task_id = t.id AND l.organization_id = t.organization_id
         WHERE t.organization_id = ?",
    );
    let mut params = vec![D1Param::Integer(organization_id)];

    if let Some(member_id) = query.member_id {
        sql.push_str(" AND t.member_id = ?");
        params.push(D1Param::Integer(member_id));
    }

    if let Some(start_date) = &query.start_date {
        sql.push_str(" AND date(datetime(l.start_at, '+9 hours')) >= ?");
        params.push(D1Param::Text(start_date.clone()));
    }

    if let Some(end_date) = &query.end_date {
        sql.push_str(" AND date(datetime(l.end_at, '+9 hours')) <= ?");
        params.push(D1Param::Text(end_date.clone()));
    }

    if let Some(raw) = &query.statuses {
        let statuses = split_csv_values(raw);
        if !statuses.is_empty() {
            let placeholders = vec!["?"; statuses.len()].join(", ");
            sql.push_str(&format!(" AND t.status IN ({placeholders})"));
            for value in statuses {
                params.push(D1Param::Text(value));
            }
        }
    }

    sql.push_str(" GROUP BY t.id, u.name ORDER BY start_at ASC, t.id ASC");

    let flat_rows = d1_query_all::<ReportFlatRow>(&state.db, &sql, &params).await?;

    let rows = flat_rows
        .into_iter()
        .map(|row| TaskReportRow {
            user_name: row.user_name,
            total_duration_minutes: row.total_duration_minutes,
            start_at: row.start_at,
            end_at: row.end_at,
            task: Task {
                id: row.id,
                organization_id: row.organization_id,
                member_id: row.member_id,
                title: row.title,
                description: row.description,
                status: row.status,
                progress_rate: row.progress_rate,
                tags: row.tags,
                created_at: row.created_at,
                updated_at: row.updated_at,
                total_duration_minutes: row.total_duration_minutes,
            },
        })
        .collect();

    Ok(rows)
}

pub async fn get_task_report(req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let result = async {
        let claims = extract_claims(&req, &ctx).await?;
        if claims.role != "admin" {
            return Err(ApiError::new(403, "Admin access required"));
        }

        let query = parse_task_report_query(&req)?;
        validate_report_date_range(&query)?;

        let rows = fetch_task_report_rows(&ctx.data, claims.organization_id, &query).await?;
        json_with_status(&rows, 200)
    }
    .await;

    result.or_else(|e| e.into_response())
}

pub async fn export_task_report(
    req: Request,
    ctx: RouteContext<AppState>,
) -> WorkerResult<Response> {
    let result = async {
        let claims = extract_claims(&req, &ctx).await?;
        if claims.role != "admin" {
            return Err(ApiError::new(403, "Admin access required"));
        }

        let query = parse_task_report_query(&req)?;
        validate_report_date_range(&query)?;

        let rows = fetch_task_report_rows(&ctx.data, claims.organization_id, &query).await?;
        let csv = task_report_to_csv(&rows);

        let mut response = Response::from_bytes(csv.into_bytes())?.with_status(200);
        let headers = response.headers_mut();
        headers.set("Content-Type", "text/csv")?;
        headers.set(
            "Content-Disposition",
            "attachment; filename=\"task_report.csv\"",
        )?;

        Ok(response)
    }
    .await;

    result.or_else(|e| e.into_response())
}
