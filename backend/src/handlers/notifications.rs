use crate::AppState;
use crate::models::{
    Claims, D1Param, D1Row, ModelError, Notification, NotificationQuery, PaginatedNotifications,
    d1_execute, d1_query_all, d1_query_one,
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

fn parse_notification_query(req: &Request) -> Result<NotificationQuery, ApiError> {
    let pairs = query_pairs(req)?;
    Ok(NotificationQuery {
        page: parse_i64_opt(pairs.get("page"), "page")?,
        per_page: parse_i64_opt(pairs.get("per_page"), "per_page")?,
    })
}

pub async fn get_notifications(
    req: Request,
    ctx: RouteContext<AppState>,
) -> WorkerResult<Response> {
    let result = async {
        let claims = extract_claims(&req, &ctx).await?;
        let query = parse_notification_query(&req)?;

        let page = query.page.unwrap_or(1).max(1);
        let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
        let offset = (page - 1) * per_page;

        let items = d1_query_all::<Notification>(
            &ctx.data.db,
            "SELECT id, organization_id, user_id, title, body, category, target_type, target_id, is_read, created_at
             FROM notifications
             WHERE organization_id = ?1
               AND user_id = ?2
               AND (is_read = 0 OR datetime(created_at) >= datetime('now', '-30 days'))
             ORDER BY is_read ASC, created_at DESC
             LIMIT ?3 OFFSET ?4",
            &[
                D1Param::Integer(claims.organization_id),
                D1Param::Integer(claims.user_id),
                D1Param::Integer(per_page),
                D1Param::Integer(offset),
            ],
        )
        .await?;

        let total = d1_query_one::<CountRow>(
            &ctx.data.db,
            "SELECT COUNT(*) AS count
             FROM notifications
             WHERE organization_id = ?1
               AND user_id = ?2
               AND (is_read = 0 OR datetime(created_at) >= datetime('now', '-30 days'))",
            &[
                D1Param::Integer(claims.organization_id),
                D1Param::Integer(claims.user_id),
            ],
        )
        .await?
        .ok_or_else(|| ApiError::internal("failed to count notifications"))?
        .count;

        let total_pages = if total == 0 {
            0
        } else {
            (total + per_page - 1) / per_page
        };

        json_with_status(
            &PaginatedNotifications {
                items,
                total,
                page,
                total_pages,
            },
            200,
        )
    }
    .await;

    result.or_else(db_error_to_response)
}

pub async fn mark_as_read(req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let result = async {
        let claims = extract_claims(&req, &ctx).await?;
        let id = ctx
            .param("id")
            .ok_or_else(|| ApiError::new(400, "Missing notification id"))?
            .parse::<i64>()
            .map_err(|_| ApiError::new(400, "Invalid notification id"))?;

        let existing = d1_query_one::<Notification>(
            &ctx.data.db,
            "SELECT id, organization_id, user_id, title, body, category, target_type, target_id, is_read, created_at
             FROM notifications
             WHERE id = ?1 AND organization_id = ?2 AND user_id = ?3
             LIMIT 1",
            &[
                D1Param::Integer(id),
                D1Param::Integer(claims.organization_id),
                D1Param::Integer(claims.user_id),
            ],
        )
        .await?;

        if existing.is_none() {
            return Err(ApiError::new(404, "Notification not found"));
        }

        d1_execute(
            &ctx.data.db,
            "UPDATE notifications
             SET is_read = 1
             WHERE id = ?1 AND organization_id = ?2 AND user_id = ?3",
            &[
                D1Param::Integer(id),
                D1Param::Integer(claims.organization_id),
                D1Param::Integer(claims.user_id),
            ],
        )
        .await?;

        let notification = d1_query_one::<Notification>(
            &ctx.data.db,
            "SELECT id, organization_id, user_id, title, body, category, target_type, target_id, is_read, created_at
             FROM notifications
             WHERE id = ?1 AND organization_id = ?2 AND user_id = ?3
             LIMIT 1",
            &[
                D1Param::Integer(id),
                D1Param::Integer(claims.organization_id),
                D1Param::Integer(claims.user_id),
            ],
        )
        .await?
        .ok_or_else(|| ApiError::internal("failed to resolve updated notification"))?;

        json_with_status(&notification, 200)
    }
    .await;

    result.or_else(db_error_to_response)
}

pub async fn mark_all_as_read(req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let result = async {
        let claims = extract_claims(&req, &ctx).await?;

        let unread = d1_query_one::<CountRow>(
            &ctx.data.db,
            "SELECT COUNT(*) AS count
             FROM notifications
             WHERE organization_id = ?1 AND user_id = ?2 AND is_read = 0",
            &[
                D1Param::Integer(claims.organization_id),
                D1Param::Integer(claims.user_id),
            ],
        )
        .await?
        .ok_or_else(|| ApiError::internal("failed to count unread notifications"))?
        .count;

        d1_execute(
            &ctx.data.db,
            "UPDATE notifications
             SET is_read = 1
             WHERE organization_id = ?1 AND user_id = ?2 AND is_read = 0",
            &[
                D1Param::Integer(claims.organization_id),
                D1Param::Integer(claims.user_id),
            ],
        )
        .await?;

        json_with_status(&json!({ "updated": unread }), 200)
    }
    .await;

    result.or_else(db_error_to_response)
}
