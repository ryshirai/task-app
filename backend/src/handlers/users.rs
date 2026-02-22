use crate::AppState;
use crate::handlers::log_activity;
use crate::models::*;
use crate::utils::{is_secure_password, is_valid_username};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::Utc;

pub async fn get_users(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<GetUsersQuery>,
) -> Result<Json<Vec<UserWithTimeLogs>>, (StatusCode, String)> {
    let date = params.date.unwrap_or_else(|| {
        let offset = chrono::FixedOffset::east_opt(9 * 3600).unwrap();
        Utc::now().with_timezone(&offset).date_naive()
    });

    let users = sqlx::query_as::<_, User>("SELECT id, organization_id, name, username, email, avatar_url, role FROM users WHERE organization_id = $1 ORDER BY id")
        .bind(claims.organization_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut result = Vec::new();
    for user in users {
        let time_logs = sqlx::query_as::<_, TaskTimeLog>(
            "SELECT l.id, l.organization_id, l.user_id, l.task_id, l.start_at, l.end_at, l.duration_minutes::BIGINT AS duration_minutes,
                    t.title AS task_title, t.description AS task_description, t.status AS task_status, t.progress_rate AS task_progress_rate,
                    ARRAY_REMOVE(ARRAY_AGG(DISTINCT tg.name), NULL) AS task_tags,
                    COALESCE(sums.total, 0)::BIGINT AS total_duration_minutes
             FROM task_time_logs l
             JOIN tasks t ON t.id = l.task_id AND t.organization_id = l.organization_id
             LEFT JOIN task_tags tt ON t.id = tt.task_id
             LEFT JOIN tags tg ON tt.tag_id = tg.id
             LEFT JOIN (
                 SELECT task_id, SUM(duration_minutes) as total 
                 FROM task_time_logs 
                 GROUP BY task_id
             ) sums ON sums.task_id = l.task_id
             WHERE l.organization_id = $1 AND l.user_id = $2 AND (l.start_at AT TIME ZONE 'Asia/Tokyo')::date = $3
             GROUP BY l.id, t.id, sums.total
             ORDER BY l.start_at ASC, l.id ASC"
        )
            .bind(claims.organization_id)
            .bind(user.id)
            .bind(date)
            .fetch_all(&state.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        result.push(UserWithTimeLogs { user, time_logs });
    }

    Ok(Json(result))
}

pub async fn update_password(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(input): Json<UpdatePasswordInput>,
) -> Result<StatusCode, (StatusCode, String)> {
    if !is_secure_password(&input.new_password) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Password must be at least 8 characters and include uppercase, lowercase, number, and symbol".to_string(),
        ));
    }

    let stored_hash: String = sqlx::query_scalar(
        "SELECT password_hash FROM users WHERE id = $1 AND organization_id = $2",
    )
    .bind(claims.user_id)
    .bind(claims.organization_id)
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
        .verify_password(input.current_password.as_bytes(), &parsed_hash)
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                "Current password is incorrect".to_string(),
            )
        })?;

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
    )
    .await;

    Ok(StatusCode::OK)
}

pub async fn create_user(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(input): Json<CreateUserInput>,
) -> Result<(StatusCode, Json<User>), (StatusCode, String)> {
    if !is_valid_username(&input.username) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Username must contain only alphanumeric characters, underscores, or hyphens"
                .to_string(),
        ));
    }
    if !is_secure_password(&input.password) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Password must be at least 8 characters and include uppercase, lowercase, number, and symbol".to_string(),
        ));
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

pub async fn update_user_role(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(target_user_id): Path<i32>,
    Json(input): Json<UpdateUserRoleInput>,
) -> Result<StatusCode, (StatusCode, String)> {
    if claims.role != "admin" {
        return Err((
            StatusCode::FORBIDDEN,
            "Only admins can update user roles".to_string(),
        ));
    }

    if claims.user_id == target_user_id {
        return Err((
            StatusCode::FORBIDDEN,
            "You cannot update your own role".to_string(),
        ));
    }

    let previous_role = sqlx::query_scalar::<_, String>(
        "SELECT role FROM users WHERE id = $1 AND organization_id = $2",
    )
    .bind(target_user_id)
    .bind(claims.organization_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    sqlx::query("UPDATE users SET role = $1 WHERE id = $2 AND organization_id = $3")
        .bind(&input.role)
        .bind(target_user_id)
        .bind(claims.organization_id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    log_activity(
        &state.pool,
        claims.organization_id,
        claims.user_id,
        "user_role_updated",
        "user",
        Some(target_user_id),
        Some(format!("role: {} -> {}", previous_role, input.role)),
    )
    .await;

    Ok(StatusCode::OK)
}
