use crate::AppState;
use crate::models::{
    ActivityLog, Claims, D1Param, D1Row, LogQuery, ModelError, PaginatedLogs, d1_query_all,
    d1_query_one,
};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::Serialize;
use serde_json::Value;
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

fn json_with_status<T: Serialize>(value: &T, status: u16) -> Result<Response, ApiError> {
    Response::from_json(value)
        .map(|response| response.with_status(status))
        .map_err(ApiError::from)
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

fn parse_log_query(req: &Request) -> Result<LogQuery, ApiError> {
    let pairs = query_pairs(req)?;
    Ok(LogQuery {
        page: parse_i64_opt(pairs.get("page"), "page")?,
        per_page: parse_i64_opt(pairs.get("per_page"), "per_page")?,
        user_id: parse_i64_opt(pairs.get("user_id"), "user_id")?,
        start_date: pairs
            .get("start_date")
            .and_then(|v| (!v.trim().is_empty()).then_some(v.clone())),
        end_date: pairs
            .get("end_date")
            .and_then(|v| (!v.trim().is_empty()).then_some(v.clone())),
        action: pairs
            .get("action")
            .and_then(|v| (!v.trim().is_empty()).then_some(v.clone())),
        target_type: pairs
            .get("target_type")
            .and_then(|v| (!v.trim().is_empty()).then_some(v.clone())),
    })
}

fn validate_date_range(query: &LogQuery) -> Result<(), ApiError> {
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

fn append_log_filters(sql: &mut String, params: &mut Vec<D1Param>, query: &LogQuery, org_id: i64) {
    sql.push_str(" WHERE l.organization_id = ?");
    params.push(D1Param::Integer(org_id));

    if let Some(user_id) = query.user_id {
        sql.push_str(" AND l.user_id = ?");
        params.push(D1Param::Integer(user_id));
    }

    if let Some(start_date) = &query.start_date {
        sql.push_str(" AND date(l.created_at) >= ?");
        params.push(D1Param::Text(start_date.clone()));
    }

    if let Some(end_date) = &query.end_date {
        sql.push_str(" AND date(l.created_at) <= ?");
        params.push(D1Param::Text(end_date.clone()));
    }

    if let Some(action) = &query.action {
        sql.push_str(" AND l.action = ?");
        params.push(D1Param::Text(action.clone()));
    }

    if let Some(target_type) = &query.target_type {
        sql.push_str(" AND l.target_type = ?");
        params.push(D1Param::Text(target_type.clone()));
    }
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
        let date = csv_escape(&log.created_at);
        let user = csv_escape(&log.user_name);
        let action = csv_escape(&log.action);
        let target_type = csv_escape(&log.target_type);
        let target_id = csv_escape(&log.target_id.map(|id| id.to_string()).unwrap_or_default());
        let details = csv_escape(log.details.as_deref().unwrap_or(""));
        csv.push_str(&format!(
            "{date},{user},{action},{target_type},{target_id},{details}\n"
        ));
    }

    csv
}

pub async fn get_logs(req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let result: Result<Response, ApiError> = async {
        let claims = extract_claims(&req, &ctx).await?;
        let query = parse_log_query(&req)?;
        validate_date_range(&query)?;

        let page = query.page.unwrap_or(1).max(1);
        let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
        let offset = (page - 1) * per_page;

        let mut sql = String::from(
            "SELECT l.id, l.organization_id, l.user_id, u.name AS user_name,
                    l.action, l.target_type, l.target_id, l.details, l.created_at
             FROM activity_logs l
             JOIN users u ON l.user_id = u.id",
        );
        let mut params = Vec::new();
        append_log_filters(&mut sql, &mut params, &query, claims.organization_id);
        sql.push_str(" ORDER BY l.created_at DESC LIMIT ? OFFSET ?");
        params.push(D1Param::Integer(per_page));
        params.push(D1Param::Integer(offset));

        let items = d1_query_all::<ActivityLog>(&ctx.data.db, &sql, &params).await?;

        let mut total_sql = String::from("SELECT COUNT(*) AS count FROM activity_logs l");
        let mut total_params = Vec::new();
        append_log_filters(
            &mut total_sql,
            &mut total_params,
            &query,
            claims.organization_id,
        );

        let total = d1_query_one::<CountRow>(&ctx.data.db, &total_sql, &total_params)
            .await?
            .ok_or_else(|| ApiError::internal("failed to count activity logs"))?
            .count;

        let total_pages = if total == 0 {
            0
        } else {
            (total + per_page - 1) / per_page
        };

        json_with_status(
            &PaginatedLogs {
                items,
                total,
                page,
                total_pages,
            },
            200,
        )
    }
    .await;

    result.or_else(|e| e.into_response())
}

pub async fn export_logs(req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let result: Result<Response, ApiError> = async {
        let claims = extract_claims(&req, &ctx).await?;
        let query = parse_log_query(&req)?;
        validate_date_range(&query)?;

        let mut sql = String::from(
            "SELECT l.id, l.organization_id, l.user_id, u.name AS user_name,
                    l.action, l.target_type, l.target_id, l.details, l.created_at
             FROM activity_logs l
             JOIN users u ON l.user_id = u.id",
        );
        let mut params = Vec::new();
        append_log_filters(&mut sql, &mut params, &query, claims.organization_id);
        sql.push_str(" ORDER BY l.created_at DESC");

        let items = d1_query_all::<ActivityLog>(&ctx.data.db, &sql, &params).await?;
        let csv = logs_to_csv(&items);

        let mut response = Response::from_bytes(csv.into_bytes())?.with_status(200);
        let headers = response.headers_mut();
        headers.set("Content-Type", "text/csv")?;
        headers.set(
            "Content-Disposition",
            "attachment; filename=\"activity_logs.csv\"",
        )?;

        Ok(response)
    }
    .await;

    result.or_else(|e| e.into_response())
}
