use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    Json, Extension,
};
use chrono::Utc;
use crate::models::*;
use crate::utils::is_valid_username;
use crate::AppState;
use crate::handlers::log_activity;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

pub async fn get_users(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<GetUsersQuery>,
) -> Result<Json<Vec<UserWithTasks>>, (StatusCode, String)> {
    let date = params.date.unwrap_or_else(|| Utc::now().naive_utc().date());

    let users = sqlx::query_as::<_, User>("SELECT id, organization_id, name, username, email, avatar_url, role FROM users WHERE organization_id = $1 ORDER BY id")
        .bind(claims.organization_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut result = Vec::new();
    for user in users {
        let tasks = sqlx::query_as::<_, Task>(
            "SELECT * FROM tasks WHERE organization_id = $1 AND member_id = $2 AND start_at::date = $3"
        )
            .bind(claims.organization_id)
            .bind(user.id)
            .bind(date)
            .fetch_all(&state.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        result.push(UserWithTasks { user, tasks });
    }

    Ok(Json(result))
}

pub async fn update_password(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(input): Json<UpdatePasswordInput>,
) -> Result<StatusCode, (StatusCode, String)> {
    let stored_hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1 AND organization_id = $2")
        .bind(claims.user_id)
        .bind(claims.organization_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let parsed_hash = PasswordHash::new(&stored_hash)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Invalid password hash in DB".to_string()))?;
    
    Argon2::default()
        .verify_password(input.current_password.as_bytes(), &parsed_hash)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Current password is incorrect".to_string()))?;

    let salt = SaltString::generate(&mut OsRng);
    let new_password_hash = Argon2::default()
        .hash_password(input.new_password.as_bytes(), &salt)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .to_string();

    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2 AND organization_id = $3")
        .bind(new_password_hash)
        .bind(claims.user_id)
        .bind(claims.organization_id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    log_activity(
        &state.pool,
        claims.organization_id,
        claims.user_id,
        "password_changed",
        "user",
        Some(claims.user_id),
        None,
    ).await;

    Ok(StatusCode::OK)
}

pub async fn create_user(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(input): Json<CreateUserInput>,
) -> Result<(StatusCode, Json<User>), (StatusCode, String)> {
    if !is_valid_username(&input.username) {
        return Err((StatusCode::BAD_REQUEST, "Username must contain only alphanumeric characters, underscores, or hyphens".to_string()));
    }

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(input.password.as_bytes(), &salt)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .to_string();

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (organization_id, name, username, password_hash, avatar_url, role) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id, organization_id, name, username, email, avatar_url, role",
    )
    .bind(claims.organization_id)
    .bind(input.name)
    .bind(input.username)
    .bind(password_hash)
    .bind(input.avatar_url)
    .bind(input.role.unwrap_or_else(|| "user".to_string()))
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(user)))
}

pub async fn delete_user(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i32>,
) -> Result<StatusCode, (StatusCode, String)> {
    sqlx::query("DELETE FROM users WHERE id = $1 AND organization_id = $2")
        .bind(id)
        .bind(claims.organization_id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
