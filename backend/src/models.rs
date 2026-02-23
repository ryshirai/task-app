use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fmt;
use worker::{D1Database, D1PreparedStatement, D1Result, wasm_bindgen::JsValue};

pub type D1Row = Map<String, Value>;

#[derive(Debug)]
pub enum ModelError {
    MissingField(&'static str),
    InvalidType {
        field: &'static str,
        expected: &'static str,
    },
    InvalidValue {
        field: &'static str,
        message: String,
    },
    Worker(worker::Error),
    Serde(serde_json::Error),
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "missing field: {field}"),
            Self::InvalidType { field, expected } => {
                write!(f, "invalid type for field {field}; expected {expected}")
            }
            Self::InvalidValue { field, message } => {
                write!(f, "invalid value for field {field}: {message}")
            }
            Self::Worker(err) => write!(f, "d1 worker error: {err}"),
            Self::Serde(err) => write!(f, "serialization error: {err}"),
        }
    }
}

impl std::error::Error for ModelError {}

impl From<worker::Error> for ModelError {
    fn from(value: worker::Error) -> Self {
        Self::Worker(value)
    }
}

impl From<serde_json::Error> for ModelError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum D1Param {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
}

impl D1Param {
    fn as_js_value(&self) -> JsValue {
        match self {
            Self::Null => JsValue::NULL,
            Self::Integer(v) => JsValue::from_f64(*v as f64),
            Self::Real(v) => JsValue::from_f64(*v),
            Self::Text(v) => JsValue::from_str(v),
        }
    }
}

pub trait FromD1Row: Sized {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError>;
}

pub trait ToD1Params {
    fn to_d1_params(&self) -> Vec<D1Param>;
}

pub async fn d1_query_all<T: FromD1Row>(
    db: &D1Database,
    sql: &str,
    params: &[D1Param],
) -> Result<Vec<T>, ModelError> {
    let mut stmt: D1PreparedStatement = db.prepare(sql);
    if !params.is_empty() {
        let js_params: Vec<JsValue> = params.iter().map(D1Param::as_js_value).collect();
        stmt = stmt.bind(&js_params)?;
    }

    let raw: D1Result = stmt.all().await?;
    let rows: Vec<Value> = raw.results::<Value>()?;
    rows.into_iter()
        .map(|value| match value {
            Value::Object(map) => T::from_d1_row(&map),
            _ => Err(ModelError::InvalidType {
                field: "row",
                expected: "object",
            }),
        })
        .collect()
}

pub async fn d1_query_one<T: FromD1Row>(
    db: &D1Database,
    sql: &str,
    params: &[D1Param],
) -> Result<Option<T>, ModelError> {
    let mut rows = d1_query_all::<T>(db, sql, params).await?;
    Ok(rows.drain(..1).next())
}

pub async fn d1_execute(db: &D1Database, sql: &str, params: &[D1Param]) -> Result<u64, ModelError> {
    let mut stmt: D1PreparedStatement = db.prepare(sql);
    if !params.is_empty() {
        let js_params: Vec<JsValue> = params.iter().map(D1Param::as_js_value).collect();
        stmt = stmt.bind(&js_params)?;
    }

    let _ = stmt.run().await?;
    Ok(0)
}

fn required_i64(row: &D1Row, field: &'static str) -> Result<i64, ModelError> {
    let value = row.get(field).ok_or(ModelError::MissingField(field))?;
    match value {
        Value::Number(n) => n.as_i64().ok_or(ModelError::InvalidType {
            field,
            expected: "integer",
        }),
        Value::String(s) => s.parse::<i64>().map_err(|_| ModelError::InvalidType {
            field,
            expected: "integer",
        }),
        _ => Err(ModelError::InvalidType {
            field,
            expected: "integer",
        }),
    }
}

fn optional_i64(row: &D1Row, field: &'static str) -> Result<Option<i64>, ModelError> {
    match row.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n.as_i64().map(Some).ok_or(ModelError::InvalidType {
            field,
            expected: "integer",
        }),
        Some(Value::String(s)) => s
            .parse::<i64>()
            .map(Some)
            .map_err(|_| ModelError::InvalidType {
                field,
                expected: "integer",
            }),
        Some(_) => Err(ModelError::InvalidType {
            field,
            expected: "integer",
        }),
    }
}

fn required_text(row: &D1Row, field: &'static str) -> Result<String, ModelError> {
    row.get(field)
        .ok_or(ModelError::MissingField(field))?
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or(ModelError::InvalidType {
            field,
            expected: "text",
        })
}

fn optional_text(row: &D1Row, field: &'static str) -> Result<Option<String>, ModelError> {
    match row.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(v)) => Ok(Some(v.clone())),
        Some(_) => Err(ModelError::InvalidType {
            field,
            expected: "text",
        }),
    }
}

fn optional_text_vec(row: &D1Row, field: &'static str) -> Result<Option<Vec<String>>, ModelError> {
    match row.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .map(ToOwned::to_owned)
                    .ok_or(ModelError::InvalidType {
                        field,
                        expected: "array<string>",
                    })
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
        Some(Value::String(raw)) => {
            if raw.trim().is_empty() {
                return Ok(Some(vec![]));
            }

            match serde_json::from_str::<Vec<String>>(raw) {
                Ok(parsed) => Ok(Some(parsed)),
                Err(_) => Ok(Some(raw.split(',').map(|s| s.trim().to_string()).collect())),
            }
        }
        Some(_) => Err(ModelError::InvalidType {
            field,
            expected: "array<string>|json-string|csv-string",
        }),
    }
}

fn required_bool_int(row: &D1Row, field: &'static str) -> Result<i64, ModelError> {
    let value = required_i64(row, field)?;
    if value == 0 || value == 1 {
        Ok(value)
    } else {
        Err(ModelError::InvalidValue {
            field,
            message: "expected 0 or 1".to_string(),
        })
    }
}

fn optional_bool_int(row: &D1Row, field: &'static str) -> Result<Option<i64>, ModelError> {
    let value = optional_i64(row, field)?;
    match value {
        None => Ok(None),
        Some(v) if v == 0 || v == 1 => Ok(Some(v)),
        Some(_) => Err(ModelError::InvalidValue {
            field,
            message: "expected 0 or 1".to_string(),
        }),
    }
}

// =============================
// Database Entities
// =============================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct User {
    pub id: i64,
    pub organization_id: i64,
    pub name: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub pending_email: Option<String>,
    pub avatar_url: Option<String>,
    pub role: String,
    pub email_verified: i64,
    pub created_at: Option<String>,
}

impl FromD1Row for User {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError> {
        Ok(Self {
            id: required_i64(row, "id")?,
            organization_id: required_i64(row, "organization_id")?,
            name: required_text(row, "name")?,
            username: optional_text(row, "username")?,
            email: optional_text(row, "email")?,
            pending_email: optional_text(row, "pending_email")?,
            avatar_url: optional_text(row, "avatar_url")?,
            role: required_text(row, "role")?,
            email_verified: required_bool_int(row, "email_verified")?,
            created_at: optional_text(row, "created_at")?,
        })
    }
}

impl ToD1Params for User {
    fn to_d1_params(&self) -> Vec<D1Param> {
        vec![
            D1Param::Integer(self.organization_id),
            D1Param::Text(self.name.clone()),
            self.username
                .as_ref()
                .map(|v| D1Param::Text(v.clone()))
                .unwrap_or(D1Param::Null),
            self.email
                .as_ref()
                .map(|v| D1Param::Text(v.clone()))
                .unwrap_or(D1Param::Null),
            self.pending_email
                .as_ref()
                .map(|v| D1Param::Text(v.clone()))
                .unwrap_or(D1Param::Null),
            self.avatar_url
                .as_ref()
                .map(|v| D1Param::Text(v.clone()))
                .unwrap_or(D1Param::Null),
            D1Param::Text(self.role.clone()),
            D1Param::Integer(self.email_verified),
        ]
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Task {
    pub id: i64,
    pub organization_id: i64,
    pub member_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub progress_rate: i64,
    pub tags: Option<Vec<String>>,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub total_duration_minutes: i64,
}

impl FromD1Row for Task {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError> {
        Ok(Self {
            id: required_i64(row, "id")?,
            organization_id: required_i64(row, "organization_id")?,
            member_id: required_i64(row, "member_id")?,
            title: required_text(row, "title")?,
            description: optional_text(row, "description")?,
            status: required_text(row, "status")?,
            progress_rate: required_i64(row, "progress_rate")?,
            tags: optional_text_vec(row, "tags")?,
            created_at: required_text(row, "created_at")?,
            updated_at: optional_text(row, "updated_at")?,
            total_duration_minutes: optional_i64(row, "total_duration_minutes")?.unwrap_or(0),
        })
    }
}

impl ToD1Params for Task {
    fn to_d1_params(&self) -> Vec<D1Param> {
        vec![
            D1Param::Integer(self.organization_id),
            D1Param::Integer(self.member_id),
            D1Param::Text(self.title.clone()),
            self.description
                .as_ref()
                .map(|v| D1Param::Text(v.clone()))
                .unwrap_or(D1Param::Null),
            D1Param::Text(self.status.clone()),
            D1Param::Integer(self.progress_rate),
        ]
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DisplayGroup {
    pub id: i64,
    pub organization_id: i64,
    pub user_id: i64,
    pub name: String,
    pub member_ids: Vec<i64>,
    pub created_at: String,
}

impl FromD1Row for DisplayGroup {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError> {
        let member_ids = match row.get("member_ids") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(values)) => values
                .iter()
                .map(|v| {
                    v.as_i64().ok_or(ModelError::InvalidType {
                        field: "member_ids",
                        expected: "array<integer>",
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
            Some(Value::String(raw)) => {
                if raw.trim().is_empty() {
                    vec![]
                } else if let Ok(parsed) = serde_json::from_str::<Vec<i64>>(raw) {
                    parsed
                } else {
                    raw.split(',')
                        .map(|v| {
                            v.trim()
                                .parse::<i64>()
                                .map_err(|_| ModelError::InvalidType {
                                    field: "member_ids",
                                    expected: "json-array|csv",
                                })
                        })
                        .collect::<Result<Vec<_>, _>>()?
                }
            }
            Some(_) => {
                return Err(ModelError::InvalidType {
                    field: "member_ids",
                    expected: "array<integer>|json-string|csv-string",
                });
            }
        };

        Ok(Self {
            id: required_i64(row, "id")?,
            organization_id: required_i64(row, "organization_id")?,
            user_id: required_i64(row, "user_id")?,
            name: required_text(row, "name")?,
            member_ids,
            created_at: required_text(row, "created_at")?,
        })
    }
}

impl ToD1Params for DisplayGroup {
    fn to_d1_params(&self) -> Vec<D1Param> {
        vec![
            D1Param::Integer(self.organization_id),
            D1Param::Integer(self.user_id),
            D1Param::Text(self.name.clone()),
        ]
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskTimeLog {
    pub id: i64,
    pub organization_id: i64,
    pub user_id: i64,
    pub task_id: i64,
    pub start_at: String,
    pub end_at: String,
    pub duration_minutes: i64,
    pub created_at: Option<String>,
    pub task_title: Option<String>,
    pub task_description: Option<String>,
    pub task_status: Option<String>,
    pub task_progress_rate: Option<i64>,
    pub task_tags: Option<Vec<String>>,
    pub total_duration_minutes: i64,
}

impl FromD1Row for TaskTimeLog {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError> {
        Ok(Self {
            id: required_i64(row, "id")?,
            organization_id: required_i64(row, "organization_id")?,
            user_id: required_i64(row, "user_id")?,
            task_id: required_i64(row, "task_id")?,
            start_at: required_text(row, "start_at")?,
            end_at: required_text(row, "end_at")?,
            duration_minutes: optional_i64(row, "duration_minutes")?.unwrap_or(0),
            created_at: optional_text(row, "created_at")?,
            task_title: optional_text(row, "task_title")?,
            task_description: optional_text(row, "task_description")?,
            task_status: optional_text(row, "task_status")?,
            task_progress_rate: optional_i64(row, "task_progress_rate")?,
            task_tags: optional_text_vec(row, "task_tags")?,
            total_duration_minutes: optional_i64(row, "total_duration_minutes")?.unwrap_or(0),
        })
    }
}

impl ToD1Params for TaskTimeLog {
    fn to_d1_params(&self) -> Vec<D1Param> {
        vec![
            D1Param::Integer(self.organization_id),
            D1Param::Integer(self.user_id),
            D1Param::Integer(self.task_id),
            D1Param::Text(self.start_at.clone()),
            D1Param::Text(self.end_at.clone()),
        ]
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskReportRow {
    pub task: Task,
    pub user_name: String,
    pub total_duration_minutes: i64,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ActivityLog {
    pub id: i64,
    pub organization_id: i64,
    pub user_id: i64,
    pub user_name: String,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<i64>,
    pub details: Option<String>,
    pub created_at: String,
}

impl FromD1Row for ActivityLog {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError> {
        Ok(Self {
            id: required_i64(row, "id")?,
            organization_id: required_i64(row, "organization_id")?,
            user_id: required_i64(row, "user_id")?,
            user_name: required_text(row, "user_name")?,
            action: required_text(row, "action")?,
            target_type: required_text(row, "target_type")?,
            target_id: optional_i64(row, "target_id")?,
            details: optional_text(row, "details")?,
            created_at: required_text(row, "created_at")?,
        })
    }
}

impl ToD1Params for ActivityLog {
    fn to_d1_params(&self) -> Vec<D1Param> {
        vec![
            D1Param::Integer(self.organization_id),
            D1Param::Integer(self.user_id),
            D1Param::Text(self.action.clone()),
            D1Param::Text(self.target_type.clone()),
            self.target_id
                .map(D1Param::Integer)
                .unwrap_or(D1Param::Null),
            self.details
                .as_ref()
                .map(|v| D1Param::Text(v.clone()))
                .unwrap_or(D1Param::Null),
        ]
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DailyReport {
    pub id: i64,
    pub organization_id: i64,
    pub user_id: i64,
    pub report_date: String,
    pub content: String,
    pub created_at: String,
}

impl FromD1Row for DailyReport {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError> {
        Ok(Self {
            id: required_i64(row, "id")?,
            organization_id: required_i64(row, "organization_id")?,
            user_id: required_i64(row, "user_id")?,
            report_date: required_text(row, "report_date")?,
            content: required_text(row, "content")?,
            created_at: required_text(row, "created_at")?,
        })
    }
}

impl ToD1Params for DailyReport {
    fn to_d1_params(&self) -> Vec<D1Param> {
        vec![
            D1Param::Integer(self.organization_id),
            D1Param::Integer(self.user_id),
            D1Param::Text(self.report_date.clone()),
            D1Param::Text(self.content.clone()),
        ]
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Invitation {
    pub id: i64,
    pub organization_id: i64,
    pub org_name: Option<String>,
    pub token: String,
    pub role: String,
    pub expires_at: String,
    pub created_at: String,
}

impl FromD1Row for Invitation {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError> {
        Ok(Self {
            id: required_i64(row, "id")?,
            organization_id: required_i64(row, "organization_id")?,
            org_name: optional_text(row, "org_name")?,
            token: required_text(row, "token")?,
            role: required_text(row, "role")?,
            expires_at: required_text(row, "expires_at")?,
            created_at: required_text(row, "created_at")?,
        })
    }
}

impl ToD1Params for Invitation {
    fn to_d1_params(&self) -> Vec<D1Param> {
        vec![
            D1Param::Integer(self.organization_id),
            D1Param::Text(self.token.clone()),
            D1Param::Text(self.role.clone()),
            D1Param::Text(self.expires_at.clone()),
        ]
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Notification {
    pub id: i64,
    pub organization_id: i64,
    pub user_id: i64,
    pub title: String,
    pub body: Option<String>,
    pub category: String,
    pub target_type: Option<String>,
    pub target_id: Option<i64>,
    pub is_read: i64,
    pub created_at: String,
}

impl FromD1Row for Notification {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError> {
        Ok(Self {
            id: required_i64(row, "id")?,
            organization_id: required_i64(row, "organization_id")?,
            user_id: required_i64(row, "user_id")?,
            title: required_text(row, "title")?,
            body: optional_text(row, "body")?,
            category: required_text(row, "category")?,
            target_type: optional_text(row, "target_type")?,
            target_id: optional_i64(row, "target_id")?,
            is_read: required_bool_int(row, "is_read")?,
            created_at: required_text(row, "created_at")?,
        })
    }
}

impl ToD1Params for Notification {
    fn to_d1_params(&self) -> Vec<D1Param> {
        vec![
            D1Param::Integer(self.organization_id),
            D1Param::Integer(self.user_id),
            D1Param::Text(self.title.clone()),
            self.body
                .as_ref()
                .map(|v| D1Param::Text(v.clone()))
                .unwrap_or(D1Param::Null),
            D1Param::Text(self.category.clone()),
            self.target_type
                .as_ref()
                .map(|v| D1Param::Text(v.clone()))
                .unwrap_or(D1Param::Null),
            self.target_id
                .map(D1Param::Integer)
                .unwrap_or(D1Param::Null),
            D1Param::Integer(self.is_read),
        ]
    }
}

// =============================
// Composite API Models
// =============================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserWithTimeLogs {
    pub user: User,
    pub time_logs: Vec<TaskTimeLog>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PaginatedLogs {
    pub items: Vec<ActivityLog>,
    pub total: i64,
    pub page: i64,
    pub total_pages: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PaginatedNotifications {
    pub items: Vec<Notification>,
    pub total: i64,
    pub page: i64,
    pub total_pages: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AnalyticsResponse {
    pub user_name: String,
    pub task_stats: TaskStats,
    pub report_stats: ReportStats,
    pub heatmap: Vec<HeatmapDay>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskStats {
    pub total_completed: i64,
    pub completed_this_week: i64,
    pub completed_last_week: i64,
    pub by_status: Vec<StatusCount>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StatusCount {
    pub status: String,
    pub count: i64,
}

impl FromD1Row for StatusCount {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError> {
        Ok(Self {
            status: required_text(row, "status")?,
            count: required_i64(row, "count")?,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReportStats {
    pub total_submitted: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HeatmapDay {
    pub date: String,
    pub count: i64,
}

impl FromD1Row for HeatmapDay {
    fn from_d1_row(row: &D1Row) -> Result<Self, ModelError> {
        Ok(Self {
            date: required_text(row, "date")?,
            count: required_i64(row, "count")?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub user_id: i64,
    pub organization_id: i64,
    pub role: String,
    pub exp: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoginResponse {
    pub token: String,
    pub user: User,
}

// =============================
// Request DTOs
// =============================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateDisplayGroupInput {
    pub name: String,
    pub member_ids: Vec<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateUserInput {
    pub name: String,
    pub username: String,
    pub password: String,
    pub avatar_url: Option<String>,
    pub role: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateTaskInput {
    pub member_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateTaskInput {
    pub member_id: Option<i64>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub progress_rate: Option<i64>,
    pub tags: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AddTimeLogInput {
    pub user_id: i64,
    pub task_id: Option<i64>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub start_at: String,
    pub end_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateTimeLogInput {
    pub start_at: Option<String>,
    pub end_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateReportInput {
    pub report_date: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegisterInput {
    pub organization_name: String,
    pub admin_name: String,
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateInvitationInput {
    pub email: String,
    pub role: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JoinInput {
    pub token: String,
    pub name: String,
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ForgotPasswordInput {
    pub username: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResetPasswordInput {
    pub token: String,
    pub new_password: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdatePasswordInput {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateUserRoleInput {
    pub role: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateEmailInput {
    pub email: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VerifyEmailInput {
    pub token: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateReportInput {
    pub content: String,
}

// =============================
// Query DTOs
// =============================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReportQuery {
    pub date: Option<String>,
    pub user_id: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetUsersQuery {
    pub date: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TaskReportQuery {
    pub member_id: Option<i64>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub statuses: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub user_id: Option<i64>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub action: Option<String>,
    pub target_type: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GetTasksQuery {
    pub member_id: Option<i64>,
    pub group_id: Option<i64>,
    pub q: Option<String>,
    pub date: Option<String>,
    pub status: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NotificationQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[allow(dead_code)]
fn _validate_boolean_helpers(row: &D1Row) -> Result<(Option<i64>, i64), ModelError> {
    Ok((
        optional_bool_int(row, "flag")?,
        required_bool_int(row, "is_read")?,
    ))
}
