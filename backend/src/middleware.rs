use axum::{
    extract::State,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use crate::models::Claims;
use crate::AppState;

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let auth_header = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "Missing authorization header".to_string()))?;

    let token_data = decode::<Claims>(
        auth_header,
        &DecodingKey::from_secret(state.jwt_secret.as_ref()),
        &Validation::new(Algorithm::HS256),
    ).map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid token".to_string()))?;

    req.extensions_mut().insert(token_data.claims);
    Ok(next.run(req).await)
}

pub async fn admin_only(
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let claims = req.extensions()
        .get::<Claims>()
        .ok_or((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))?;

    if claims.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Admin access required".to_string()));
    }

    Ok(next.run(req).await)
}
