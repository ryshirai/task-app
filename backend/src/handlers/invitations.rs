use crate::AppState;
use crate::models::*;
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::Utc;

pub async fn create_invitation(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(input): Json<CreateInvitationInput>,
) -> Result<(StatusCode, Json<Invitation>), (StatusCode, String)> {
    if claims.role != "admin" {
        return Err((
            StatusCode::FORBIDDEN,
            "Only admins can create invitations".to_string(),
        ));
    }

    let token = uuid::Uuid::new_v4().to_string();
    let expires_at = Utc::now() + chrono::Duration::days(7);

    let invitation = sqlx::query_as::<_, Invitation>(
        "WITH inserted AS (
            INSERT INTO invitations (organization_id, token, role, expires_at) 
            VALUES ($1, $2, $3, $4) 
            RETURNING *
        )
        SELECT i.*, o.name as org_name FROM inserted i
        JOIN organizations o ON i.organization_id = o.id",
    )
    .bind(claims.organization_id)
    .bind(token)
    .bind(input.role)
    .bind(expires_at)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(invitation)))
}

pub async fn get_invitation(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<Invitation>, (StatusCode, String)> {
    let invitation = sqlx::query_as::<_, Invitation>(
        "SELECT i.*, o.name as org_name FROM invitations i 
         JOIN organizations o ON i.organization_id = o.id 
         WHERE i.token = $1 AND i.expires_at > $2",
    )
    .bind(token)
    .bind(Utc::now())
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((
        StatusCode::NOT_FOUND,
        "Invalid or expired invitation token".to_string(),
    ))?;

    Ok(Json(invitation))
}
