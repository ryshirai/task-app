use crate::AppState;
use crate::models::{
    Claims, CreateDisplayGroupInput, D1Param, D1Row, DisplayGroup, ModelError, d1_execute,
    d1_query_all, d1_query_one,
};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::Serialize;
use serde_json::Value;
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
struct GroupExistsRow {
    #[allow(dead_code)]
    id: i64,
}

impl crate::models::FromD1Row for GroupExistsRow {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError> {
        let id = row
            .get("id")
            .and_then(Value::as_i64)
            .ok_or(ModelError::MissingField("id"))?;
        Ok(Self { id })
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

pub async fn get_display_groups(
    req: Request,
    ctx: RouteContext<AppState>,
) -> WorkerResult<Response> {
    let result = async {
        let claims = extract_claims(&req, &ctx).await?;

        let groups = d1_query_all::<DisplayGroup>(
            &ctx.data.db,
            "SELECT g.id, g.organization_id, g.user_id, g.name,
                    COALESCE(NULLIF(GROUP_CONCAT(m.member_id), ''), '') AS member_ids,
                    g.created_at
             FROM display_groups g
             LEFT JOIN display_group_members m ON g.id = m.group_id
             WHERE g.organization_id = ?1 AND g.user_id = ?2
             GROUP BY g.id
             ORDER BY g.name ASC",
            &[
                D1Param::Integer(claims.organization_id),
                D1Param::Integer(claims.user_id),
            ],
        )
        .await?;

        json_with_status(&groups, 200)
    }
    .await;

    result.or_else(db_error_to_response)
}

pub async fn create_display_group(
    mut req: Request,
    ctx: RouteContext<AppState>,
) -> WorkerResult<Response> {
    let input: CreateDisplayGroupInput = match req.json().await {
        Ok(v) => v,
        Err(e) => return ApiError::new(400, e.to_string()).into_response(),
    };

    let result = async {
        let claims = extract_claims(&req, &ctx).await?;

        d1_execute(
            &ctx.data.db,
            "INSERT INTO display_groups (organization_id, user_id, name) VALUES (?1, ?2, ?3)",
            &[
                D1Param::Integer(claims.organization_id),
                D1Param::Integer(claims.user_id),
                D1Param::Text(input.name.clone()),
            ],
        )
        .await?;

        let group = d1_query_one::<DisplayGroup>(
            &ctx.data.db,
            "SELECT id, organization_id, user_id, name,
                    '' AS member_ids,
                    created_at
             FROM display_groups
             WHERE organization_id = ?1 AND user_id = ?2 AND name = ?3
             ORDER BY id DESC
             LIMIT 1",
            &[
                D1Param::Integer(claims.organization_id),
                D1Param::Integer(claims.user_id),
                D1Param::Text(input.name.clone()),
            ],
        )
        .await?
        .ok_or_else(|| ApiError::internal("Failed to resolve created group"))?;

        for member_id in &input.member_ids {
            d1_execute(
                &ctx.data.db,
                "INSERT INTO display_group_members (group_id, member_id) VALUES (?1, ?2)",
                &[D1Param::Integer(group.id), D1Param::Integer(*member_id)],
            )
            .await?;
        }

        let result = DisplayGroup {
            member_ids: input.member_ids,
            ..group
        };

        json_with_status(&result, 201)
    }
    .await;

    result.or_else(db_error_to_response)
}

pub async fn update_display_group(
    mut req: Request,
    ctx: RouteContext<AppState>,
) -> WorkerResult<Response> {
    let input: CreateDisplayGroupInput = match req.json().await {
        Ok(v) => v,
        Err(e) => return ApiError::new(400, e.to_string()).into_response(),
    };

    let result = async {
        let claims = extract_claims(&req, &ctx).await?;
        let id = ctx
            .param("id")
            .ok_or_else(|| ApiError::new(400, "Missing group id"))?
            .parse::<i64>()
            .map_err(|_| ApiError::new(400, "Invalid group id"))?;

        let exists = d1_query_one::<GroupExistsRow>(
            &ctx.data.db,
            "SELECT id
             FROM display_groups
             WHERE id = ?1 AND organization_id = ?2 AND user_id = ?3
             LIMIT 1",
            &[
                D1Param::Integer(id),
                D1Param::Integer(claims.organization_id),
                D1Param::Integer(claims.user_id),
            ],
        )
        .await?;

        if exists.is_none() {
            return Err(ApiError::new(404, "Group not found"));
        }

        d1_execute(
            &ctx.data.db,
            "UPDATE display_groups
             SET name = ?1
             WHERE id = ?2 AND organization_id = ?3 AND user_id = ?4",
            &[
                D1Param::Text(input.name.clone()),
                D1Param::Integer(id),
                D1Param::Integer(claims.organization_id),
                D1Param::Integer(claims.user_id),
            ],
        )
        .await?;

        d1_execute(
            &ctx.data.db,
            "DELETE FROM display_group_members WHERE group_id = ?1",
            &[D1Param::Integer(id)],
        )
        .await?;

        for member_id in &input.member_ids {
            d1_execute(
                &ctx.data.db,
                "INSERT INTO display_group_members (group_id, member_id) VALUES (?1, ?2)",
                &[D1Param::Integer(id), D1Param::Integer(*member_id)],
            )
            .await?;
        }

        let mut group = d1_query_one::<DisplayGroup>(
            &ctx.data.db,
            "SELECT id, organization_id, user_id, name,
                    COALESCE(NULLIF(GROUP_CONCAT(m.member_id), ''), '') AS member_ids,
                    g.created_at
             FROM display_groups g
             LEFT JOIN display_group_members m ON g.id = m.group_id
             WHERE g.id = ?1 AND g.organization_id = ?2 AND g.user_id = ?3
             GROUP BY g.id
             LIMIT 1",
            &[
                D1Param::Integer(id),
                D1Param::Integer(claims.organization_id),
                D1Param::Integer(claims.user_id),
            ],
        )
        .await?
        .ok_or_else(|| ApiError::internal("Failed to load updated group"))?;

        group.member_ids = input.member_ids;
        json_with_status(&group, 200)
    }
    .await;

    result.or_else(db_error_to_response)
}

pub async fn delete_display_group(
    req: Request,
    ctx: RouteContext<AppState>,
) -> WorkerResult<Response> {
    let result = async {
        let claims = extract_claims(&req, &ctx).await?;
        let id = ctx
            .param("id")
            .ok_or_else(|| ApiError::new(400, "Missing group id"))?
            .parse::<i64>()
            .map_err(|_| ApiError::new(400, "Invalid group id"))?;

        let exists = d1_query_one::<GroupExistsRow>(
            &ctx.data.db,
            "SELECT id
             FROM display_groups
             WHERE id = ?1 AND organization_id = ?2 AND user_id = ?3
             LIMIT 1",
            &[
                D1Param::Integer(id),
                D1Param::Integer(claims.organization_id),
                D1Param::Integer(claims.user_id),
            ],
        )
        .await?;

        if exists.is_none() {
            return Err(ApiError::new(404, "Group not found"));
        }

        d1_execute(
            &ctx.data.db,
            "DELETE FROM display_group_members WHERE group_id = ?1",
            &[D1Param::Integer(id)],
        )
        .await?;

        d1_execute(
            &ctx.data.db,
            "DELETE FROM display_groups WHERE id = ?1 AND organization_id = ?2 AND user_id = ?3",
            &[
                D1Param::Integer(id),
                D1Param::Integer(claims.organization_id),
                D1Param::Integer(claims.user_id),
            ],
        )
        .await?;

        Ok(Response::empty()?.with_status(204))
    }
    .await;

    result.or_else(db_error_to_response)
}
