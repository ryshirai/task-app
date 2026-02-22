use crate::AppState;
use crate::models::*;
use crate::utils::{is_secure_password, is_valid_username};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use axum::{Json, extract::State, http::StatusCode};
use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header, encode};

/// JWT lifetime in hours.
const JWT_EXPIRATION_HOURS: i64 = 24;

/// Password-reset token lifetime in hours.
const PASSWORD_RESET_EXPIRATION_HOURS: i64 = 1;

/// Shared validation error for invalid credentials.
const INVALID_CREDENTIALS_MESSAGE: &str = "Invalid username or password";

/// Shared validation error for invalid usernames.
const INVALID_USERNAME_MESSAGE: &str =
    "Username must contain only alphanumeric characters, underscores, or hyphens";
const INVALID_PASSWORD_MESSAGE: &str = "Password must be at least 8 characters and include uppercase, lowercase, number, and symbol";

/// Builds JWT claims for a given authenticated user.
fn build_claims(user: &User) -> Claims {
    let expiration = Utc::now()
        .checked_add_signed(chrono::Duration::hours(JWT_EXPIRATION_HOURS))
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

/// Encodes claims into a signed JWT token.
fn encode_token(jwt_secret: &str, claims: &Claims) -> Result<String, (StatusCode, String)> {
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(jwt_secret.as_ref()),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Hashes a plaintext password with Argon2 using a random salt.
fn hash_password(password: &str) -> Result<String, (StatusCode, String)> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        .map(|hash| hash.to_string())
}

/// Authenticates a user and returns a signed JWT with user profile data.
pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginInput>,
) -> Result<Json<LoginResponse>, (StatusCode, String)> {
    // Step 1: Load the user by username or email.
    let user = sqlx::query_as::<_, User>("SELECT id, organization_id, name, username, email, avatar_url, role FROM users WHERE username = $1 OR email = $1")
        .bind(&input.username)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, INVALID_CREDENTIALS_MESSAGE.to_string()))?;

    // Step 2: Retrieve and parse the stored password hash.
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

    // Step 3: Verify the submitted password.
    Argon2::default()
        .verify_password(input.password.as_bytes(), &parsed_hash)
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                INVALID_CREDENTIALS_MESSAGE.to_string(),
            )
        })?;

    // Step 4: Build claims and sign a JWT.
    let claims = build_claims(&user);
    let token = encode_token(&state.jwt_secret, &claims)?;

    Ok(Json(LoginResponse { token, user }))
}

/// Registers a new organization and admin account, then returns an auth token.
pub async fn register(
    State(state): State<AppState>,
    Json(input): Json<RegisterInput>,
) -> Result<(StatusCode, Json<LoginResponse>), (StatusCode, String)> {
    // Step 1: Validate username format.
    if !is_valid_username(&input.username) {
        return Err((
            StatusCode::BAD_REQUEST,
            INVALID_USERNAME_MESSAGE.to_string(),
        ));
    }
    if !is_secure_password(&input.password) {
        return Err((
            StatusCode::BAD_REQUEST,
            INVALID_PASSWORD_MESSAGE.to_string(),
        ));
    }

    // Step 2: Begin transaction for organization + admin creation.
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Step 3: Create the organization record.
    let org =
        sqlx::query_as::<_, (i32,)>("INSERT INTO organizations (name) VALUES ($1) RETURNING id")
            .bind(&input.organization_name)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Step 4: Hash the admin password.
    let password_hash = hash_password(&input.password)?;

    // Step 5: Create the admin user in the new organization.
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

    // Step 6: Commit the transaction.
    tx.commit()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Step 7: Issue a token for the newly created admin.
    let claims = build_claims(&user);
    let token = encode_token(&state.jwt_secret, &claims)?;

    Ok((StatusCode::CREATED, Json(LoginResponse { token, user })))
}

/// Accepts an invitation token, creates the invited user, and returns an auth token.
pub async fn join(
    State(state): State<AppState>,
    Json(input): Json<JoinInput>,
) -> Result<(StatusCode, Json<LoginResponse>), (StatusCode, String)> {
    // Step 1: Validate username format.
    if !is_valid_username(&input.username) {
        return Err((
            StatusCode::BAD_REQUEST,
            INVALID_USERNAME_MESSAGE.to_string(),
        ));
    }
    if !is_secure_password(&input.password) {
        return Err((
            StatusCode::BAD_REQUEST,
            INVALID_PASSWORD_MESSAGE.to_string(),
        ));
    }

    // Step 2: Validate and load invitation + organization context.
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

    // Step 3: Hash the incoming password.
    let password_hash = hash_password(&input.password)?;

    // Step 4: Create the invited user.
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

    // Step 5: Best-effort invitation cleanup.
    let _ = sqlx::query("DELETE FROM invitations WHERE id = $1")
        .bind(invitation.id)
        .execute(&state.pool)
        .await;

    // Step 6: Issue a token for the newly joined user.
    let claims = build_claims(&user);
    let token = encode_token(&state.jwt_secret, &claims)?;

    Ok((StatusCode::CREATED, Json(LoginResponse { token, user })))
}

/// Creates a password-reset token for a user identified by username.
///
/// The token is stored in the database and printed to stdout for local flows.
pub async fn forgot_password(
    State(state): State<AppState>,
    Json(input): Json<ForgotPasswordInput>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Step 1: Ensure the user exists.
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&input.username)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    // Step 2: Create a reset token and expiration timestamp.
    let token = uuid::Uuid::new_v4().to_string();
    let expires_at = Utc::now() + chrono::Duration::hours(PASSWORD_RESET_EXPIRATION_HOURS);

    // Step 3: Persist reset token in the database.
    sqlx::query("INSERT INTO password_resets (user_id, token, expires_at) VALUES ($1, $2, $3)")
        .bind(user.id)
        .bind(&token)
        .bind(expires_at)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Step 4: Emit token to stdout (existing behavior).
    println!(
        "PASSWORD RESET TOKEN for {}: {}",
        user.username.unwrap_or_default(),
        token
    );

    Ok(StatusCode::OK)
}

/// Resets a user's password using a valid reset token.
pub async fn reset_password(
    State(state): State<AppState>,
    Json(input): Json<ResetPasswordInput>,
) -> Result<StatusCode, (StatusCode, String)> {
    if !is_secure_password(&input.new_password) {
        return Err((
            StatusCode::BAD_REQUEST,
            INVALID_PASSWORD_MESSAGE.to_string(),
        ));
    }

    // Step 1: Validate reset token and fetch associated user.
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

    // Step 2: Hash the new password.
    let password_hash = hash_password(&input.new_password)?;

    // Step 3: Update the user's password hash.
    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(password_hash)
        .bind(reset.1)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Step 4: Best-effort cleanup of the consumed reset token.
    let _ = sqlx::query("DELETE FROM password_resets WHERE id = $1")
        .bind(reset.0)
        .execute(&state.pool)
        .await;

    Ok(StatusCode::OK)
}
