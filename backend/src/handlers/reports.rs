use crate::AppState;
use crate::models::{
    Claims, CreateReportInput, D1Param, D1Row, DailyReport, ModelError, ReportQuery,
    UpdateReportInput, d1_execute, d1_query_all, d1_query_one,
};
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

fn parse_report_query(req: &Request) -> Result<ReportQuery, ApiError> {
    let pairs = query_pairs(req)?;
    Ok(ReportQuery {
        date: pairs.get("date").cloned(),
        user_id: parse_i64_opt(pairs.get("user_id"), "user_id")?,
    })
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

pub async fn get_reports(req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let result = async {
        let claims = extract_claims(&req, &ctx).await?;
        let params = parse_report_query(&req)?;

        let reports = d1_query_all::<DailyReport>(
            &ctx.data.db,
            "SELECT id, organization_id, user_id, report_date, content, created_at
             FROM daily_reports
             WHERE organization_id = ?1
               AND (?2 IS NULL OR report_date = ?2)
               AND (?3 IS NULL OR user_id = ?3)
             ORDER BY report_date DESC, created_at DESC",
            &[
                D1Param::Integer(claims.organization_id),
                params.date.map(D1Param::Text).unwrap_or(D1Param::Null),
                params
                    .user_id
                    .map(D1Param::Integer)
                    .unwrap_or(D1Param::Null),
            ],
        )
        .await?;

        json_with_status(&reports, 200)
    }
    .await;

    result.or_else(db_error_to_response)
}

pub async fn get_report(req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let result = async {
        let claims = extract_claims(&req, &ctx).await?;
        let id = ctx
            .param("id")
            .ok_or_else(|| ApiError::new(400, "Missing report id"))?
            .parse::<i64>()
            .map_err(|_| ApiError::new(400, "Invalid report id"))?;

        let report = d1_query_one::<DailyReport>(
            &ctx.data.db,
            "SELECT id, organization_id, user_id, report_date, content, created_at
             FROM daily_reports
             WHERE id = ?1 AND organization_id = ?2
             LIMIT 1",
            &[
                D1Param::Integer(id),
                D1Param::Integer(claims.organization_id),
            ],
        )
        .await?
        .ok_or_else(|| ApiError::new(404, "Report not found"))?;

        json_with_status(&report, 200)
    }
    .await;

    result.or_else(db_error_to_response)
}

pub async fn create_report(
    mut req: Request,
    ctx: RouteContext<AppState>,
) -> WorkerResult<Response> {
    let input: CreateReportInput = match req.json().await {
        Ok(v) => v,
        Err(e) => return ApiError::new(400, e.to_string()).into_response(),
    };

    let result = async {
        let claims = extract_claims(&req, &ctx).await?;

        d1_execute(
            &ctx.data.db,
            "INSERT INTO daily_reports (organization_id, user_id, report_date, content)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (organization_id, user_id, report_date)
             DO UPDATE SET content = excluded.content",
            &[
                D1Param::Integer(claims.organization_id),
                D1Param::Integer(claims.user_id),
                D1Param::Text(input.report_date.clone()),
                D1Param::Text(input.content.clone()),
            ],
        )
        .await?;

        let report = d1_query_one::<DailyReport>(
            &ctx.data.db,
            "SELECT id, organization_id, user_id, report_date, content, created_at
             FROM daily_reports
             WHERE organization_id = ?1 AND user_id = ?2 AND report_date = ?3
             LIMIT 1",
            &[
                D1Param::Integer(claims.organization_id),
                D1Param::Integer(claims.user_id),
                D1Param::Text(input.report_date.clone()),
            ],
        )
        .await?
        .ok_or_else(|| ApiError::internal("failed to resolve saved report"))?;

        log_activity_d1(
            &ctx.data,
            claims.organization_id,
            claims.user_id,
            "report_submitted",
            "report",
            Some(report.id),
            Some(format!("Date: {}", report.report_date)),
        )
        .await;

        json_with_status(&report, 201)
    }
    .await;

    result.or_else(db_error_to_response)
}

pub async fn update_report(
    mut req: Request,
    ctx: RouteContext<AppState>,
) -> WorkerResult<Response> {
    let input: UpdateReportInput = match req.json().await {
        Ok(v) => v,
        Err(e) => return ApiError::new(400, e.to_string()).into_response(),
    };

    let result = async {
        let claims = extract_claims(&req, &ctx).await?;
        let id = ctx
            .param("id")
            .ok_or_else(|| ApiError::new(400, "Missing report id"))?
            .parse::<i64>()
            .map_err(|_| ApiError::new(400, "Invalid report id"))?;

        let report = d1_query_one::<DailyReport>(
            &ctx.data.db,
            "SELECT id, organization_id, user_id, report_date, content, created_at
             FROM daily_reports
             WHERE id = ?1 AND organization_id = ?2
             LIMIT 1",
            &[
                D1Param::Integer(id),
                D1Param::Integer(claims.organization_id),
            ],
        )
        .await?
        .ok_or_else(|| ApiError::new(404, "Report not found"))?;

        if report.user_id != claims.user_id && claims.role != "admin" {
            return Err(ApiError::new(403, "You can only edit your own reports"));
        }

        d1_execute(
            &ctx.data.db,
            "UPDATE daily_reports
             SET content = ?1
             WHERE id = ?2 AND organization_id = ?3",
            &[
                D1Param::Text(input.content.clone()),
                D1Param::Integer(id),
                D1Param::Integer(claims.organization_id),
            ],
        )
        .await?;

        let updated_report = d1_query_one::<DailyReport>(
            &ctx.data.db,
            "SELECT id, organization_id, user_id, report_date, content, created_at
             FROM daily_reports
             WHERE id = ?1 AND organization_id = ?2
             LIMIT 1",
            &[
                D1Param::Integer(id),
                D1Param::Integer(claims.organization_id),
            ],
        )
        .await?
        .ok_or_else(|| ApiError::internal("failed to resolve updated report"))?;

        let mut changes = Vec::new();
        if report.content != updated_report.content {
            changes.push(json!({
                "field": "content",
                "old": &report.content,
                "new": &updated_report.content
            }));
        }

        log_activity_d1(
            &ctx.data,
            claims.organization_id,
            claims.user_id,
            "report_updated",
            "report",
            Some(updated_report.id),
            Some(json!({ "changes": changes }).to_string()),
        )
        .await;

        json_with_status(&updated_report, 200)
    }
    .await;

    result.or_else(db_error_to_response)
}
