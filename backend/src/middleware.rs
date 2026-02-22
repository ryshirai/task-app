use crate::AppState;
use crate::models::Claims;
use axum::{
    extract::State,
    http::{Request, StatusCode, header},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));

    let query_token = req.uri().query().and_then(|query| {
        query
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .find_map(|(key, value)| (key == "token" && !value.is_empty()).then_some(value))
    });

    let token = auth_header.or(query_token).ok_or((
        StatusCode::UNAUTHORIZED,
        "Missing authorization token".to_string(),
    ))?;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_ref()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid token".to_string()))?;

    let mut claims = token_data.claims;
    let latest_role: String = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
        .bind(claims.user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))?;
    claims.role = latest_role;

    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

pub async fn admin_only(
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let claims = req
        .extensions()
        .get::<Claims>()
        .ok_or((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))?;

    if claims.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Admin access required".to_string()));
    }

    Ok(next.run(req).await)
}
