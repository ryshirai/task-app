use crate::AppState;
use crate::models::{
    AnalyticsResponse, Claims, D1Param, D1Row, HeatmapDay, ModelError, ReportStats, StatusCount,
    TaskStats, d1_query_all, d1_query_one,
};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::Serialize;
use serde_json::Value;
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
struct NameRow {
    name: String,
}

impl crate::models::FromD1Row for NameRow {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError> {
        let name = row
            .get("name")
            .and_then(Value::as_str)
            .ok_or(ModelError::MissingField("name"))?
            .to_string();
        Ok(Self { name })
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
struct TaskCompletionStats {
    total_completed: i64,
    completed_this_week: i64,
    completed_last_week: i64,
}

impl crate::models::FromD1Row for TaskCompletionStats {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError> {
        let total_completed = row
            .get("total_completed")
            .and_then(Value::as_i64)
            .ok_or(ModelError::MissingField("total_completed"))?;
        let completed_this_week = row
            .get("completed_this_week")
            .and_then(Value::as_i64)
            .ok_or(ModelError::MissingField("completed_this_week"))?;
        let completed_last_week = row
            .get("completed_last_week")
            .and_then(Value::as_i64)
            .ok_or(ModelError::MissingField("completed_last_week"))?;

        Ok(Self {
            total_completed,
            completed_this_week,
            completed_last_week,
        })
    }
}

fn json_with_status<T: Serialize>(value: &T, status: u16) -> Result<Response, ApiError> {
    Response::from_json(value)
        .map(|response| response.with_status(status))
        .map_err(ApiError::from)
}

fn db_error_to_response(err: ApiError) -> WorkerResult<Response> {
    err.into_response()
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

async fn fetch_user_analytics(
    state: &AppState,
    organization_id: i64,
    user_id: i64,
) -> Result<AnalyticsResponse, ApiError> {
    let user_name = d1_query_one::<NameRow>(
        &state.db,
        "SELECT name FROM users WHERE organization_id = ?1 AND id = ?2 LIMIT 1",
        &[D1Param::Integer(organization_id), D1Param::Integer(user_id)],
    )
    .await?
    .ok_or_else(|| ApiError::new(404, "User not found"))?
    .name;

    let task_completion = d1_query_one::<TaskCompletionStats>(
        &state.db,
        "WITH jst AS (
             SELECT date(
                 'now',
                 '+9 hours',
                 printf('-%d days', (CAST(strftime('%w', 'now', '+9 hours') AS INTEGER) + 6) % 7)
             ) AS week_start
         )
         SELECT
             COALESCE(SUM(CASE WHEN status = 'done' THEN 1 ELSE 0 END), 0) AS total_completed,
             COALESCE(SUM(
                 CASE
                     WHEN status = 'done'
                      AND date(datetime(updated_at, '+9 hours')) >= (SELECT week_start FROM jst)
                      AND date(datetime(updated_at, '+9 hours')) < date((SELECT week_start FROM jst), '+7 days')
                     THEN 1 ELSE 0
                 END
             ), 0) AS completed_this_week,
             COALESCE(SUM(
                 CASE
                     WHEN status = 'done'
                      AND date(datetime(updated_at, '+9 hours')) >= date((SELECT week_start FROM jst), '-7 days')
                      AND date(datetime(updated_at, '+9 hours')) < (SELECT week_start FROM jst)
                     THEN 1 ELSE 0
                 END
             ), 0) AS completed_last_week
         FROM tasks
         WHERE organization_id = ?1 AND member_id = ?2",
        &[D1Param::Integer(organization_id), D1Param::Integer(user_id)],
    )
    .await?
    .ok_or_else(|| ApiError::internal("failed to compute task completion stats"))?;

    let by_status = d1_query_all::<StatusCount>(
        &state.db,
        "SELECT status, COUNT(*) AS count
         FROM tasks
         WHERE organization_id = ?1 AND member_id = ?2
         GROUP BY status
         ORDER BY count DESC, status ASC",
        &[D1Param::Integer(organization_id), D1Param::Integer(user_id)],
    )
    .await?;

    let total_reports = d1_query_one::<CountRow>(
        &state.db,
        "SELECT COUNT(*) AS count
         FROM daily_reports
         WHERE organization_id = ?1 AND user_id = ?2",
        &[D1Param::Integer(organization_id), D1Param::Integer(user_id)],
    )
    .await?
    .ok_or_else(|| ApiError::internal("failed to count reports"))?
    .count;

    let heatmap = d1_query_all::<HeatmapDay>(
        &state.db,
        "WITH RECURSIVE days(day) AS (
             SELECT date('now', '+9 hours', '-29 days')
             UNION ALL
             SELECT date(day, '+1 day')
             FROM days
             WHERE day < date('now', '+9 hours')
         )
         SELECT
             day AS date,
             COALESCE(COUNT(al.id), 0) AS count
         FROM days
         LEFT JOIN activity_logs al
             ON al.organization_id = ?1
            AND al.user_id = ?2
            AND date(datetime(al.created_at, '+9 hours')) = day
         GROUP BY day
         ORDER BY day ASC",
        &[D1Param::Integer(organization_id), D1Param::Integer(user_id)],
    )
    .await?;

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
    req: Request,
    ctx: RouteContext<AppState>,
) -> WorkerResult<Response> {
    let result = async {
        let claims = extract_claims(&req, &ctx).await?;
        let analytics =
            fetch_user_analytics(&ctx.data, claims.organization_id, claims.user_id).await?;
        json_with_status(&analytics, 200)
    }
    .await;

    result.or_else(db_error_to_response)
}

pub async fn get_user_analytics(
    req: Request,
    ctx: RouteContext<AppState>,
) -> WorkerResult<Response> {
    let result = async {
        let claims = extract_claims(&req, &ctx).await?;
        let id = ctx
            .param("id")
            .ok_or_else(|| ApiError::new(400, "Missing user id"))?
            .parse::<i64>()
            .map_err(|_| ApiError::new(400, "Invalid user id"))?;

        if claims.role != "admin" && claims.user_id != id {
            return Err(ApiError::new(403, "Forbidden"));
        }

        let analytics = fetch_user_analytics(&ctx.data, claims.organization_id, id).await?;
        json_with_status(&analytics, 200)
    }
    .await;

    result.or_else(db_error_to_response)
}
