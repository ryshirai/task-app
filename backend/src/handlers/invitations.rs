use crate::AppState;
use crate::models::{
    Claims, CreateInvitationInput, D1Param, D1Row, Invitation, ModelError, d1_execute, d1_query_one,
};
use chrono::{Duration, Utc};
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

pub async fn create_invitation(
    mut req: Request,
    ctx: RouteContext<AppState>,
) -> WorkerResult<Response> {
    let input: CreateInvitationInput = match req.json().await {
        Ok(v) => v,
        Err(e) => return ApiError::new(400, e.to_string()).into_response(),
    };

    let result = async {
        let claims = extract_claims(&req, &ctx).await?;
        if claims.role != "admin" {
            return Err(ApiError::new(403, "Only admins can create invitations"));
        }

        let token = uuid::Uuid::new_v4().to_string();
        let expires_at = (Utc::now() + Duration::days(7))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        d1_execute(
            &ctx.data.db,
            "INSERT INTO invitations (organization_id, token, role, expires_at)
             VALUES (?1, ?2, ?3, ?4)",
            &[
                D1Param::Integer(claims.organization_id),
                D1Param::Text(token.clone()),
                D1Param::Text(input.role.clone()),
                D1Param::Text(expires_at),
            ],
        )
        .await?;

        let invitation = d1_query_one::<Invitation>(
            &ctx.data.db,
            "SELECT i.id, i.organization_id, o.name AS org_name, i.token, i.role, i.expires_at, i.created_at
             FROM invitations i
             JOIN organizations o ON i.organization_id = o.id
             WHERE i.token = ?1
             LIMIT 1",
            &[D1Param::Text(token.clone())],
        )
        .await?
        .ok_or_else(|| ApiError::internal("Failed to resolve created invitation"))?;

        if let Some(email) = &input.email {
            ctx.data
                .email_service
                .send_invitation_email(
                    email,
                    &invitation.token,
                    invitation
                        .org_name
                        .as_deref()
                        .unwrap_or("Your Organization"),
                )
                .await
                .map_err(ApiError::internal)?;
        }

        json_with_status(&invitation, 201)
    }
    .await;

    result.or_else(db_error_to_response)
}

pub async fn get_invitation(_req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let result = async {
        let token = ctx
            .param("token")
            .ok_or_else(|| ApiError::new(400, "Missing invitation token"))?
            .to_string();

        let invitation = d1_query_one::<Invitation>(
            &ctx.data.db,
            "SELECT i.id, i.organization_id, o.name AS org_name, i.token, i.role, i.expires_at, i.created_at
             FROM invitations i
             JOIN organizations o ON i.organization_id = o.id
             WHERE i.token = ?1
               AND datetime(i.expires_at) > datetime('now')
             LIMIT 1",
            &[D1Param::Text(token)],
        )
        .await?
        .ok_or_else(|| ApiError::new(404, "Invalid or expired invitation token"))?;

        json_with_status(&invitation, 200)
    }
    .await;

    result.or_else(db_error_to_response)
}
