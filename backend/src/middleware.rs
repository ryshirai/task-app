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
        .and_then(|h| h.strip_prefix("Bearer "));

    let query_token = req.uri()
        .query()
        .and_then(|query| {
            query
                .split('&')
                .filter_map(|pair| pair.split_once('='))
                .find_map(|(key, value)| (key == "token" && !value.is_empty()).then_some(value))
        });

    let token = auth_header
        .or(query_token)
        .ok_or((StatusCode::UNAUTHORIZED, "Missing authorization token".to_string()))?;

    let token_data = decode::<Claims>(
        token,
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
