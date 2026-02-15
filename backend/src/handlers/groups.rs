use crate::AppState;
use crate::models::*;
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};

pub async fn get_display_groups(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<DisplayGroup>>, (StatusCode, String)> {
    let groups = sqlx::query_as::<_, DisplayGroup>(
        "SELECT g.*, COALESCE(ARRAY_AGG(m.member_id), ARRAY[]::integer[]) as member_ids
         FROM display_groups g
         LEFT JOIN display_group_members m ON g.id = m.group_id
         WHERE g.organization_id = $1 AND g.user_id = $2
         GROUP BY g.id
         ORDER BY g.name ASC"
    )
    .bind(claims.organization_id)
    .bind(claims.user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(groups))
}

pub async fn create_display_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(input): Json<CreateDisplayGroupInput>,
) -> Result<(StatusCode, Json<DisplayGroup>), (StatusCode, String)> {
    let mut tx = state.pool.begin().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let group = sqlx::query_as::<_, DisplayGroup>(
        "INSERT INTO display_groups (organization_id, user_id, name) VALUES ($1, $2, $3) RETURNING *"
    )
    .bind(claims.organization_id)
    .bind(claims.user_id)
    .bind(&input.name)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    for member_id in &input.member_ids {
        sqlx::query("INSERT INTO display_group_members (group_id, member_id) VALUES ($1, $2)")
            .bind(group.id)
            .bind(member_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    tx.commit().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut result = group;
    result.member_ids = input.member_ids;

    Ok((StatusCode::CREATED, Json(result)))
}

pub async fn update_display_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i32>,
    Json(input): Json<CreateDisplayGroupInput>,
) -> Result<Json<DisplayGroup>, (StatusCode, String)> {
    let mut tx = state.pool.begin().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Check ownership
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM display_groups WHERE id = $1 AND user_id = $2)"
    )
    .bind(id)
    .bind(claims.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !exists {
        return Err((StatusCode::NOT_FOUND, "Group not found".to_string()));
    }

    let group = sqlx::query_as::<_, DisplayGroup>(
        "UPDATE display_groups SET name = $1 WHERE id = $2 RETURNING *"
    )
    .bind(&input.name)
    .bind(id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    sqlx::query("DELETE FROM display_group_members WHERE group_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    for member_id in &input.member_ids {
        sqlx::query("INSERT INTO display_group_members (group_id, member_id) VALUES ($1, $2)")
            .bind(id)
            .bind(member_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    tx.commit().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut result = group;
    result.member_ids = input.member_ids;

    Ok(Json(result))
}

pub async fn delete_display_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<i32>,
) -> Result<StatusCode, (StatusCode, String)> {
    let deleted = sqlx::query("DELETE FROM display_groups WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(claims.user_id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if deleted.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Group not found".to_string()));
    }

    Ok(StatusCode::NO_CONTENT)
}
