use crate::AppState;
use crate::models::*;
use crate::utils::is_valid_username;
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use axum::{Json, extract::State, http::StatusCode};
use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header, encode};

pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginInput>,
) -> Result<Json<LoginResponse>, (StatusCode, String)> {
    let user = sqlx::query_as::<_, User>("SELECT id, organization_id, name, username, email, avatar_url, role FROM users WHERE username = $1")
        .bind(&input.username)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid username or password".to_string()))?;

    let stored_hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
        .bind(user.id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let parsed_hash = PasswordHash::new(&stored_hash).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid password hash in DB".to_string(),
        )
    })?;

    Argon2::default()
        .verify_password(input.password.as_bytes(), &parsed_hash)
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                "Invalid username or password".to_string(),
            )
        })?;

    let expiration = Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user.username.clone().unwrap_or_default(),
        user_id: user.id,
        organization_id: user.organization_id,
        role: user.role.clone(),
        exp: expiration,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_ref()),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(LoginResponse { token, user }))
}

pub async fn register(
    State(state): State<AppState>,
    Json(input): Json<RegisterInput>,
) -> Result<(StatusCode, Json<LoginResponse>), (StatusCode, String)> {
    if !is_valid_username(&input.username) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Username must contain only alphanumeric characters, underscores, or hyphens"
                .to_string(),
        ));
    }

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let org =
        sqlx::query_as::<_, (i32,)>("INSERT INTO organizations (name) VALUES ($1) RETURNING id")
            .bind(&input.organization_name)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(input.password.as_bytes(), &salt)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .to_string();

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (organization_id, name, username, email, password_hash, role) VALUES ($1, $2, $3, $4, $5, 'admin') RETURNING id, organization_id, name, username, email, avatar_url, role",
    )
    .bind(org.0)
    .bind(&input.admin_name)
    .bind(&input.username)
    .bind(&input.email)
    .bind(password_hash)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let expiration = Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user.username.clone().unwrap_or_default(),
        user_id: user.id,
        organization_id: user.organization_id,
        role: user.role.clone(),
        exp: expiration,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_ref()),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(LoginResponse { token, user })))
}

pub async fn join(
    State(state): State<AppState>,
    Json(input): Json<JoinInput>,
) -> Result<(StatusCode, Json<LoginResponse>), (StatusCode, String)> {
    if !is_valid_username(&input.username) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Username must contain only alphanumeric characters, underscores, or hyphens"
                .to_string(),
        ));
    }

    let invitation = sqlx::query_as::<_, Invitation>(
        "SELECT i.*, o.name as org_name FROM invitations i 
         JOIN organizations o ON i.organization_id = o.id 
         WHERE i.token = $1 AND i.expires_at > $2",
    )
    .bind(&input.token)
    .bind(Utc::now())
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((
        StatusCode::NOT_FOUND,
        "Invalid or expired invitation token".to_string(),
    ))?;

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(input.password.as_bytes(), &salt)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .to_string();

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (organization_id, name, username, email, password_hash, role) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id, organization_id, name, username, email, avatar_url, role",
    )
    .bind(invitation.organization_id)
    .bind(input.name)
    .bind(input.username)
    .bind(input.email)
    .bind(password_hash)
    .bind(invitation.role)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let _ = sqlx::query("DELETE FROM invitations WHERE id = $1")
        .bind(invitation.id)
        .execute(&state.pool)
        .await;

    let expiration = Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user.username.clone().unwrap_or_default(),
        user_id: user.id,
        organization_id: user.organization_id,
        role: user.role.clone(),
        exp: expiration,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_ref()),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(LoginResponse { token, user })))
}

pub async fn forgot_password(
    State(state): State<AppState>,
    Json(input): Json<ForgotPasswordInput>,
) -> Result<StatusCode, (StatusCode, String)> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&input.username)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let token = uuid::Uuid::new_v4().to_string();
    let expires_at = Utc::now() + chrono::Duration::hours(1);

    sqlx::query("INSERT INTO password_resets (user_id, token, expires_at) VALUES ($1, $2, $3)")
        .bind(user.id)
        .bind(&token)
        .bind(expires_at)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    println!(
        "PASSWORD RESET TOKEN for {}: {}",
        user.username.unwrap_or_default(),
        token
    );

    Ok(StatusCode::OK)
}

pub async fn reset_password(
    State(state): State<AppState>,
    Json(input): Json<ResetPasswordInput>,
) -> Result<StatusCode, (StatusCode, String)> {
    let reset = sqlx::query_as::<_, (i32, i32)>(
        "SELECT id, user_id FROM password_resets WHERE token = $1 AND expires_at > $2",
    )
    .bind(&input.token)
    .bind(Utc::now())
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((
        StatusCode::NOT_FOUND,
        "Invalid or expired reset token".to_string(),
    ))?;

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(input.new_password.as_bytes(), &salt)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .to_string();

    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(password_hash)
        .bind(reset.1)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let _ = sqlx::query("DELETE FROM password_resets WHERE id = $1")
        .bind(reset.0)
        .execute(&state.pool)
        .await;

    Ok(StatusCode::OK)
}
