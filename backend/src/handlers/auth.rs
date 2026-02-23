use crate::AppState;
use crate::models::{
    Claims, D1Param, D1Row, ForgotPasswordInput, Invitation, JoinInput, LoginInput, LoginResponse,
    ModelError, RegisterInput, ResetPasswordInput, User, VerifyEmailInput, d1_execute,
    d1_query_one,
};
use crate::utils::{is_secure_password, is_valid_username};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::Serialize;
use serde_json::{Value, json};
use worker::{Request, Response, Result as WorkerResult, RouteContext};

const JWT_EXPIRATION_HOURS: i64 = 24;
const PASSWORD_RESET_EXPIRATION_HOURS: i64 = 1;

const INVALID_CREDENTIALS_MESSAGE: &str = "Invalid username or password";
const INVALID_USERNAME_MESSAGE: &str =
    "Username must contain only alphanumeric characters, underscores, or hyphens";
const INVALID_PASSWORD_MESSAGE: &str =
    "Password must be at least 8 characters and include uppercase, lowercase, number, and symbol";

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
struct UserPasswordRow {
    password_hash: String,
}

impl crate::models::FromD1Row for UserPasswordRow {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError> {
        let password_hash = row
            .get("password_hash")
            .and_then(Value::as_str)
            .ok_or(ModelError::MissingField("password_hash"))?
            .to_string();
        Ok(Self { password_hash })
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
struct ResetRow {
    id: i64,
    user_id: i64,
}

impl crate::models::FromD1Row for ResetRow {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError> {
        let id = row
            .get("id")
            .and_then(Value::as_i64)
            .ok_or(ModelError::MissingField("id"))?;
        let user_id = row
            .get("user_id")
            .and_then(Value::as_i64)
            .ok_or(ModelError::MissingField("user_id"))?;
        Ok(Self { id, user_id })
    }
}

#[derive(Clone, Debug)]
struct PendingEmailRow {
    pending_email: Option<String>,
}

impl crate::models::FromD1Row for PendingEmailRow {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError> {
        let pending_email = match row.get("pending_email") {
            None | Some(Value::Null) => None,
            Some(Value::String(v)) => Some(v.clone()),
            _ => {
                return Err(ModelError::InvalidType {
                    field: "pending_email",
                    expected: "text|null",
                });
            }
        };

        Ok(Self { pending_email })
    }
}

fn build_claims(user: &User) -> Claims {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(JWT_EXPIRATION_HOURS))
        .expect("valid timestamp")
        .timestamp() as usize;

    Claims {
        sub: user.username.clone().unwrap_or_default(),
        user_id: user.id,
        organization_id: user.organization_id,
        role: user.role.clone(),
        exp: expiration,
    }
}

fn encode_token(jwt_secret: &str, claims: &Claims) -> Result<String, ApiError> {
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(jwt_secret.as_ref()),
    )
    .map_err(|e| ApiError::internal(e.to_string()))
}

fn hash_password(password: &str) -> Result<String, ApiError> {
    // Avoid `OsRng` in Workers by deriving a per-hash salt from UUID bytes.
    let salt = SaltString::encode_b64(uuid::Uuid::new_v4().as_bytes())
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| ApiError::internal(e.to_string()))
        .map(|hash| hash.to_string())
}

fn json_with_status<T: Serialize>(value: &T, status: u16) -> Result<Response, ApiError> {
    Response::from_json(value)
        .map(|response| response.with_status(status))
        .map_err(ApiError::from)
}

fn db_error_to_response(err: ApiError) -> WorkerResult<Response> {
    err.into_response()
}

pub async fn login(mut req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let input: LoginInput = match req.json().await {
        Ok(v) => v,
        Err(e) => return ApiError::new(400, e.to_string()).into_response(),
    };

    let result = async {
        let user = d1_query_one::<User>(
            &ctx.data.db,
            "SELECT id, organization_id, name, username, email, pending_email, avatar_url, role, email_verified, created_at
             FROM users
             WHERE username = ?1 OR email = ?1
             LIMIT 1",
            &[D1Param::Text(input.username.clone())],
        )
        .await?
        .ok_or_else(|| ApiError::new(401, INVALID_CREDENTIALS_MESSAGE))?;

        let stored_hash = d1_query_one::<UserPasswordRow>(
            &ctx.data.db,
            "SELECT password_hash FROM users WHERE id = ?1 LIMIT 1",
            &[D1Param::Integer(user.id)],
        )
        .await?
        .ok_or_else(|| ApiError::internal("Missing password hash"))?;

        let parsed_hash = PasswordHash::new(&stored_hash.password_hash)
            .map_err(|_| ApiError::internal("Invalid password hash in DB"))?;

        Argon2::default()
            .verify_password(input.password.as_bytes(), &parsed_hash)
            .map_err(|_| ApiError::new(401, INVALID_CREDENTIALS_MESSAGE))?;

        let claims = build_claims(&user);
        let token = encode_token(&ctx.data.jwt_secret, &claims)?;

        json_with_status(&LoginResponse { token, user }, 200)
    }
    .await;

    result.or_else(db_error_to_response)
}

pub async fn register(mut req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let input: RegisterInput = match req.json().await {
        Ok(v) => v,
        Err(e) => return ApiError::new(400, e.to_string()).into_response(),
    };

    let result = async {
        if !is_valid_username(&input.username) {
            return Err(ApiError::new(400, INVALID_USERNAME_MESSAGE));
        }
        if !is_secure_password(&input.password) {
            return Err(ApiError::new(400, INVALID_PASSWORD_MESSAGE));
        }

        d1_execute(
            &ctx.data.db,
            "INSERT INTO organizations (name) VALUES (?1)",
            &[D1Param::Text(input.organization_name.clone())],
        )
        .await?;

        let org = d1_query_one::<IdRow>(
            &ctx.data.db,
            "SELECT id FROM organizations WHERE name = ?1 ORDER BY id DESC LIMIT 1",
            &[D1Param::Text(input.organization_name.clone())],
        )
        .await?
        .ok_or_else(|| ApiError::internal("Failed to load created organization"))?;

        let password_hash = hash_password(&input.password)?;

        d1_execute(
            &ctx.data.db,
            "INSERT INTO users (organization_id, name, username, email, password_hash, role, email_verified)
             VALUES (?1, ?2, ?3, ?4, ?5, 'admin', 1)",
            &[
                D1Param::Integer(org.id),
                D1Param::Text(input.admin_name.clone()),
                D1Param::Text(input.username.clone()),
                D1Param::Text(input.email.clone()),
                D1Param::Text(password_hash),
            ],
        )
        .await?;

        let user = d1_query_one::<User>(
            &ctx.data.db,
            "SELECT id, organization_id, name, username, email, pending_email, avatar_url, role, email_verified, created_at
             FROM users
             WHERE organization_id = ?1 AND username = ?2
             LIMIT 1",
            &[
                D1Param::Integer(org.id),
                D1Param::Text(input.username.clone()),
            ],
        )
        .await?
        .ok_or_else(|| ApiError::internal("Failed to load created user"))?;

        let claims = build_claims(&user);
        let token = encode_token(&ctx.data.jwt_secret, &claims)?;

        json_with_status(&LoginResponse { token, user }, 201)
    }
    .await;

    result.or_else(db_error_to_response)
}

pub async fn join(mut req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let input: JoinInput = match req.json().await {
        Ok(v) => v,
        Err(e) => return ApiError::new(400, e.to_string()).into_response(),
    };

    let result = async {
        if !is_valid_username(&input.username) {
            return Err(ApiError::new(400, INVALID_USERNAME_MESSAGE));
        }
        if !is_secure_password(&input.password) {
            return Err(ApiError::new(400, INVALID_PASSWORD_MESSAGE));
        }

        let invitation = d1_query_one::<Invitation>(
            &ctx.data.db,
            "SELECT i.id, i.organization_id, o.name AS org_name, i.token, i.role, i.expires_at, i.created_at
             FROM invitations i
             JOIN organizations o ON i.organization_id = o.id
             WHERE i.token = ?1 AND datetime(i.expires_at) > datetime('now')
             LIMIT 1",
            &[D1Param::Text(input.token.clone())],
        )
        .await?
        .ok_or_else(|| ApiError::new(404, "Invalid or expired invitation token"))?;

        let password_hash = hash_password(&input.password)?;

        d1_execute(
            &ctx.data.db,
            "INSERT INTO users (organization_id, name, username, email, password_hash, role, email_verified)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
            &[
                D1Param::Integer(invitation.organization_id),
                D1Param::Text(input.name.clone()),
                D1Param::Text(input.username.clone()),
                D1Param::Text(input.email.clone()),
                D1Param::Text(password_hash),
                D1Param::Text(invitation.role.clone()),
            ],
        )
        .await?;

        let user = d1_query_one::<User>(
            &ctx.data.db,
            "SELECT id, organization_id, name, username, email, pending_email, avatar_url, role, email_verified, created_at
             FROM users
             WHERE organization_id = ?1 AND username = ?2
             LIMIT 1",
            &[
                D1Param::Integer(invitation.organization_id),
                D1Param::Text(input.username.clone()),
            ],
        )
        .await?
        .ok_or_else(|| ApiError::internal("Failed to load joined user"))?;

        let _ = d1_execute(
            &ctx.data.db,
            "DELETE FROM invitations WHERE id = ?1",
            &[D1Param::Integer(invitation.id)],
        )
        .await;

        let claims = build_claims(&user);
        let token = encode_token(&ctx.data.jwt_secret, &claims)?;

        json_with_status(&LoginResponse { token, user }, 201)
    }
    .await;

    result.or_else(db_error_to_response)
}

pub async fn forgot_password(
    mut req: Request,
    ctx: RouteContext<AppState>,
) -> WorkerResult<Response> {
    let input: ForgotPasswordInput = match req.json().await {
        Ok(v) => v,
        Err(e) => return ApiError::new(400, e.to_string()).into_response(),
    };

    let result = async {
        let user = d1_query_one::<User>(
            &ctx.data.db,
            "SELECT id, organization_id, name, username, email, pending_email, avatar_url, role, email_verified, created_at
             FROM users
             WHERE username = ?1
             LIMIT 1",
            &[D1Param::Text(input.username.clone())],
        )
        .await?
        .ok_or_else(|| ApiError::new(404, "User not found"))?;

        let token = uuid::Uuid::new_v4().to_string();
        let expires_at = (Utc::now() + Duration::hours(PASSWORD_RESET_EXPIRATION_HOURS)).to_rfc3339();

        d1_execute(
            &ctx.data.db,
            "INSERT INTO password_resets (user_id, token, expires_at) VALUES (?1, ?2, ?3)",
            &[
                D1Param::Integer(user.id),
                D1Param::Text(token.clone()),
                D1Param::Text(expires_at),
            ],
        )
        .await?;

        let recipient = user
            .email
            .clone()
            .or_else(|| user.username.clone())
            .ok_or_else(|| {
                ApiError::new(
                    400,
                    "User has no email or username to send reset instructions",
                )
            })?;

        ctx.data
            .email_service
            .send_password_reset_email(&recipient, &token)
            .await
            .map_err(ApiError::internal)?;

        json_with_status(&json!({ "status": "ok" }), 200)
    }
    .await;

    result.or_else(db_error_to_response)
}

pub async fn reset_password(
    mut req: Request,
    ctx: RouteContext<AppState>,
) -> WorkerResult<Response> {
    let input: ResetPasswordInput = match req.json().await {
        Ok(v) => v,
        Err(e) => return ApiError::new(400, e.to_string()).into_response(),
    };

    let result = async {
        if !is_secure_password(&input.new_password) {
            return Err(ApiError::new(400, INVALID_PASSWORD_MESSAGE));
        }

        let reset = d1_query_one::<ResetRow>(
            &ctx.data.db,
            "SELECT id, user_id
             FROM password_resets
             WHERE token = ?1 AND datetime(expires_at) > datetime('now')
             LIMIT 1",
            &[D1Param::Text(input.token.clone())],
        )
        .await?
        .ok_or_else(|| ApiError::new(404, "Invalid or expired reset token"))?;

        let password_hash = hash_password(&input.new_password)?;

        d1_execute(
            &ctx.data.db,
            "UPDATE users SET password_hash = ?1 WHERE id = ?2",
            &[
                D1Param::Text(password_hash),
                D1Param::Integer(reset.user_id),
            ],
        )
        .await?;

        let _ = d1_execute(
            &ctx.data.db,
            "DELETE FROM password_resets WHERE id = ?1",
            &[D1Param::Integer(reset.id)],
        )
        .await;

        json_with_status(&json!({ "status": "ok" }), 200)
    }
    .await;

    result.or_else(db_error_to_response)
}

pub async fn verify_email(mut req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let input: VerifyEmailInput = match req.json().await {
        Ok(v) => v,
        Err(e) => return ApiError::new(400, e.to_string()).into_response(),
    };

    let result = async {
        let pending_email = d1_query_one::<PendingEmailRow>(
            &ctx.data.db,
            "SELECT pending_email FROM users WHERE email_verification_token = ?1 LIMIT 1",
            &[D1Param::Text(input.token.clone())],
        )
        .await?
        .ok_or_else(|| ApiError::new(404, "Invalid or expired verification token"))?;

        if pending_email.pending_email.is_none() {
            return Err(ApiError::new(400, "No pending email to verify"));
        }

        d1_execute(
            &ctx.data.db,
            "UPDATE users
             SET email = pending_email,
                 pending_email = NULL,
                 email_verified = 1,
                 email_verification_token = NULL
             WHERE email_verification_token = ?1",
            &[D1Param::Text(input.token.clone())],
        )
        .await?;

        json_with_status(&json!({ "status": "ok" }), 200)
    }
    .await;

    result.or_else(db_error_to_response)
}
