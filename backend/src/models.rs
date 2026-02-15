use chrono::{DateTime, Utc, NaiveDate};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, sqlx::FromRow, Clone, Debug)]
pub struct User {
    pub id: i32,
    pub organization_id: i32,
    pub name: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub role: String,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Task {
    pub id: i32,
    pub organization_id: i32,
    pub member_id: i32,
    pub title: String,
    pub status: String,
    pub progress_rate: i32,
    pub tags: Option<Vec<String>>,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct ActivityLog {
    pub id: i32,
    pub organization_id: i32,
    pub user_id: i32,
    pub user_name: String,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<i32>,
    pub details: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct UserWithTasks {
    #[serde(flatten)]
    pub user: User,
    pub tasks: Vec<Task>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct DailyReport {
    pub id: i32,
    pub organization_id: i32,
    pub user_id: i32,
    pub report_date: NaiveDate,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: User,
}

#[derive(Deserialize)]
pub struct CreateUserInput {
    pub name: String,
    pub username: String,
    pub password: String,
    pub avatar_url: Option<String>,
    pub role: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateTaskInput {
    pub member_id: i32,
    pub title: String,
    pub tags: Option<Vec<String>>,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CreateReportInput {
    pub report_date: NaiveDate,
    pub content: String,
}

#[derive(Deserialize)]
pub struct RegisterInput {
    pub organization_name: String,
    pub admin_name: String,
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct CreateInvitationInput {
    pub role: String,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Invitation {
    pub id: i32,
    pub organization_id: i32,
    pub org_name: Option<String>,
    pub token: String,
    pub role: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct JoinInput {
    pub token: String,
    pub name: String,
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct ForgotPasswordInput {
    pub username: String,
}

#[derive(Deserialize)]
pub struct ResetPasswordInput {
    pub token: String,
    pub new_password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub user_id: i32,
    pub organization_id: i32,
    pub role: String,
    pub exp: usize,
}

#[derive(Deserialize)]
pub struct UpdatePasswordInput {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Deserialize)]
pub struct UpdateTaskInput {
    pub title: Option<String>,
    pub status: Option<String>,
    pub progress_rate: Option<i32>,
    pub tags: Option<Vec<String>>,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct UpdateReportInput {
    pub content: String,
}

#[derive(Deserialize)]
pub struct ReportQuery {
    pub date: Option<NaiveDate>,
    pub user_id: Option<i32>,
}

#[derive(Deserialize)]
pub struct GetUsersQuery {
    pub date: Option<NaiveDate>,
}

#[derive(Deserialize)]
pub struct LogQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub user_id: Option<i32>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub action: Option<String>,
    pub target_type: Option<String>,
}

#[derive(Serialize)]
pub struct PaginatedLogs {
    pub items: Vec<ActivityLog>,
    pub total: i64,
    pub page: i64,
    pub total_pages: i64,
}
