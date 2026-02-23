use crate::AppState;
use crate::models::{
    Claims, CreateUserInput, D1Param, D1Row, GetUsersQuery, ModelError, TaskTimeLog,
    UpdateEmailInput, UpdatePasswordInput, UpdateUserRoleInput, User, UserWithTimeLogs, d1_execute,
    d1_query_all, d1_query_one,
};
use crate::utils::{is_secure_password, is_valid_username};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use chrono::Utc;
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

impl From<serde_json::Error> for ApiError {
    fn from(value: serde_json::Error) -> Self {
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
struct PasswordRow {
    password_hash: String,
}

impl crate::models::FromD1Row for PasswordRow {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError> {
        let password_hash = row
            .get("password_hash")
            .and_then(Value::as_str)
            .ok_or(ModelError::MissingField("password_hash"))?
            .to_string();
        Ok(Self { password_hash })
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
    for (k, v) in url.query_pairs() {
        pairs.insert(k.into_owned(), v.into_owned());
    }
    Ok(pairs)
}

fn parse_get_users_query(req: &Request) -> Result<GetUsersQuery, ApiError> {
    let pairs = query_pairs(req)?;
    Ok(GetUsersQuery {
        date: pairs.get("date").cloned(),
    })
}

fn today_jst_date() -> String {
    let offset = chrono::FixedOffset::east_opt(9 * 3600).expect("valid fixed offset");
    Utc::now()
        .with_timezone(&offset)
        .date_naive()
        .format("%Y-%m-%d")
        .to_string()
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

pub async fn get_users(req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let result = async {
        let claims = extract_claims(&req, &ctx).await?;
        let params = parse_get_users_query(&req)?;
        let date = params.date.unwrap_or_else(today_jst_date);

        let users = d1_query_all::<User>(
            &ctx.data.db,
            "SELECT id, organization_id, name, username, email, pending_email, avatar_url, role, email_verified, created_at
             FROM users
             WHERE organization_id = ?1
             ORDER BY id",
            &[D1Param::Integer(claims.organization_id)],
        )
        .await?;

        let mut result = Vec::with_capacity(users.len());

        for user in users {
            let time_logs = d1_query_all::<TaskTimeLog>(
                &ctx.data.db,
                "SELECT l.id, l.organization_id, l.user_id, l.task_id, l.start_at, l.end_at, l.duration_minutes,
                        l.created_at,
                        t.title AS task_title, t.description AS task_description, t.status AS task_status,
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
                     WHERE organization_id = ?1
                     GROUP BY task_id
                 ) sums ON sums.task_id = l.task_id
                 WHERE l.organization_id = ?1
                   AND l.user_id = ?2
                   AND date(datetime(l.start_at, '+9 hours')) = ?3
                 GROUP BY l.id
                 ORDER BY l.start_at ASC, l.id ASC",
                &[
                    D1Param::Integer(claims.organization_id),
                    D1Param::Integer(user.id),
                    D1Param::Text(date.clone()),
                ],
            )
            .await?;

            result.push(UserWithTimeLogs { user, time_logs });
        }

        let mut final_result = Vec::new();
        for entry in result {
            let mut val = serde_json::to_value(entry.user).map_err(ApiError::from)?;
            if let Value::Object(ref mut map) = val {
                map.insert(
                    "time_logs".to_string(),
                    serde_json::to_value(entry.time_logs).map_err(ApiError::from)?,
                );
            }
            final_result.push(val);
        }
        json_with_status(&final_result, 200)
    }
    .await;

    result.or_else(db_error_to_response)
}

pub async fn update_password(
    mut req: Request,
    ctx: RouteContext<AppState>,
) -> WorkerResult<Response> {
    let input: UpdatePasswordInput = match req.json().await {
        Ok(v) => v,
        Err(e) => return ApiError::new(400, e.to_string()).into_response(),
    };

    let result = async {
        let claims = extract_claims(&req, &ctx).await?;

        if !is_secure_password(&input.new_password) {
            return Err(ApiError::new(
                400,
                "Password must be at least 8 characters and include uppercase, lowercase, number, and symbol",
            ));
        }

        let stored_hash = d1_query_one::<PasswordRow>(
            &ctx.data.db,
            "SELECT password_hash
             FROM users
             WHERE id = ?1 AND organization_id = ?2
             LIMIT 1",
            &[
                D1Param::Integer(claims.user_id),
                D1Param::Integer(claims.organization_id),
            ],
        )
        .await?
        .ok_or_else(|| ApiError::new(404, "User not found"))?;

        let parsed_hash = PasswordHash::new(&stored_hash.password_hash)
            .map_err(|_| ApiError::internal("Invalid password hash in DB"))?;

        Argon2::default()
            .verify_password(input.current_password.as_bytes(), &parsed_hash)
            .map_err(|_| ApiError::new(401, "Current password is incorrect"))?;

        let salt = SaltString::encode_b64(uuid::Uuid::new_v4().as_bytes())
            .map_err(|e| ApiError::internal(e.to_string()))?;
        let new_password_hash = Argon2::default()
            .hash_password(input.new_password.as_bytes(), &salt)
            .map_err(|e| ApiError::internal(e.to_string()))?
            .to_string();

        d1_execute(
            &ctx.data.db,
            "UPDATE users
             SET password_hash = ?1
             WHERE id = ?2 AND organization_id = ?3",
            &[
                D1Param::Text(new_password_hash),
                D1Param::Integer(claims.user_id),
                D1Param::Integer(claims.organization_id),
            ],
        )
        .await?;

        log_activity_d1(
            &ctx.data,
            claims.organization_id,
            claims.user_id,
            "password_changed",
            "user",
            Some(claims.user_id),
            None,
        )
        .await;

        json_with_status(&json!({ "status": "ok" }), 200)
    }
    .await;

    result.or_else(db_error_to_response)
}

pub async fn create_user(mut req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let input: CreateUserInput = match req.json().await {
        Ok(v) => v,
        Err(e) => return ApiError::new(400, e.to_string()).into_response(),
    };

    let result = async {
        let claims = extract_claims(&req, &ctx).await?;

        if claims.role != "admin" {
            return Err(ApiError::new(403, "Only admins can create users"));
        }

        if !is_valid_username(&input.username) {
            return Err(ApiError::new(
                400,
                "Username must contain only alphanumeric characters, underscores, or hyphens",
            ));
        }
        if !is_secure_password(&input.password) {
            return Err(ApiError::new(
                400,
                "Password must be at least 8 characters and include uppercase, lowercase, number, and symbol",
            ));
        }

        let salt = SaltString::encode_b64(uuid::Uuid::new_v4().as_bytes())
            .map_err(|e| ApiError::internal(e.to_string()))?;
        let password_hash = Argon2::default()
            .hash_password(input.password.as_bytes(), &salt)
            .map_err(|e| ApiError::internal(e.to_string()))?
            .to_string();

        d1_execute(
            &ctx.data.db,
            "INSERT INTO users (organization_id, name, username, password_hash, avatar_url, role, email_verified)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
            &[
                D1Param::Integer(claims.organization_id),
                D1Param::Text(input.name.clone()),
                D1Param::Text(input.username.clone()),
                D1Param::Text(password_hash),
                input
                    .avatar_url
                    .clone()
                    .map(D1Param::Text)
                    .unwrap_or(D1Param::Null),
                D1Param::Text(input.role.clone().unwrap_or_else(|| "user".to_string())),
            ],
        )
        .await?;

        let user = d1_query_one::<User>(
            &ctx.data.db,
            "SELECT id, organization_id, name, username, email, pending_email, avatar_url, role, email_verified, created_at
             FROM users
             WHERE organization_id = ?1 AND username = ?2
             ORDER BY id DESC
             LIMIT 1",
            &[
                D1Param::Integer(claims.organization_id),
                D1Param::Text(input.username.clone()),
            ],
        )
        .await?
        .ok_or_else(|| ApiError::internal("Failed to load created user"))?;

        json_with_status(&user, 201)
    }
    .await;

    result.or_else(db_error_to_response)
}

pub async fn delete_user(req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let result = async {
        let claims = extract_claims(&req, &ctx).await?;

        if claims.role != "admin" {
            return Err(ApiError::new(403, "Only admins can delete users"));
        }

        let id = ctx
            .param("id")
            .ok_or_else(|| ApiError::new(400, "Missing user id"))?
            .parse::<i64>()
            .map_err(|_| ApiError::new(400, "Invalid user id"))?;

        d1_execute(
            &ctx.data.db,
            "DELETE FROM users WHERE id = ?1 AND organization_id = ?2",
            &[
                D1Param::Integer(id),
                D1Param::Integer(claims.organization_id),
            ],
        )
        .await?;

        Ok(Response::empty()?.with_status(204))
    }
    .await;

    result.or_else(db_error_to_response)
}

pub async fn update_user_role(
    mut req: Request,
    ctx: RouteContext<AppState>,
) -> WorkerResult<Response> {
    let input: UpdateUserRoleInput = match req.json().await {
        Ok(v) => v,
        Err(e) => return ApiError::new(400, e.to_string()).into_response(),
    };

    let result = async {
        let claims = extract_claims(&req, &ctx).await?;

        let target_user_id = ctx
            .param("id")
            .ok_or_else(|| ApiError::new(400, "Missing user id"))?
            .parse::<i64>()
            .map_err(|_| ApiError::new(400, "Invalid user id"))?;

        if claims.role != "admin" {
            return Err(ApiError::new(403, "Only admins can update user roles"));
        }

        if claims.user_id == target_user_id {
            return Err(ApiError::new(403, "You cannot update your own role"));
        }

        let previous_role = d1_query_one::<RoleRow>(
            &ctx.data.db,
            "SELECT role FROM users WHERE id = ?1 AND organization_id = ?2 LIMIT 1",
            &[
                D1Param::Integer(target_user_id),
                D1Param::Integer(claims.organization_id),
            ],
        )
        .await?
        .ok_or_else(|| ApiError::new(404, "User not found"))?;

        d1_execute(
            &ctx.data.db,
            "UPDATE users SET role = ?1 WHERE id = ?2 AND organization_id = ?3",
            &[
                D1Param::Text(input.role.clone()),
                D1Param::Integer(target_user_id),
                D1Param::Integer(claims.organization_id),
            ],
        )
        .await?;

        log_activity_d1(
            &ctx.data,
            claims.organization_id,
            claims.user_id,
            "user_role_updated",
            "user",
            Some(target_user_id),
            Some(format!("role: {} -> {}", previous_role.role, input.role)),
        )
        .await;

        json_with_status(&json!({ "status": "ok" }), 200)
    }
    .await;

    result.or_else(db_error_to_response)
}

pub async fn update_email(mut req: Request, ctx: RouteContext<AppState>) -> WorkerResult<Response> {
    let input: UpdateEmailInput = match req.json().await {
        Ok(v) => v,
        Err(e) => return ApiError::new(400, e.to_string()).into_response(),
    };

    let result = async {
        let claims = extract_claims(&req, &ctx).await?;

        if !crate::utils::is_valid_email(&input.email) {
            return Err(ApiError::new(400, "Invalid email format"));
        }

        let token = uuid::Uuid::new_v4().to_string();

        d1_execute(
            &ctx.data.db,
            "UPDATE users
             SET pending_email = ?1, email_verification_token = ?2
             WHERE id = ?3 AND organization_id = ?4",
            &[
                D1Param::Text(input.email.clone()),
                D1Param::Text(token.clone()),
                D1Param::Integer(claims.user_id),
                D1Param::Integer(claims.organization_id),
            ],
        )
        .await?;

        ctx.data
            .email_service
            .send_verification_email(&input.email, &token)
            .await
            .map_err(ApiError::internal)?;

        log_activity_d1(
            &ctx.data,
            claims.organization_id,
            claims.user_id,
            "update_email",
            "user",
            Some(claims.user_id),
            Some(format!("Changed email to {}", input.email)),
        )
        .await;

        json_with_status(&json!({ "status": "ok" }), 200)
    }
    .await;

    result.or_else(db_error_to_response)
}
