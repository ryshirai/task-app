use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

// =============================
// Database Entities
// =============================

/// Application user entity mapped from the `users` table.
#[derive(Serialize, Deserialize, sqlx::FromRow, Clone, Debug)]
pub struct User {
    /// Primary key of the user.
    pub id: i32,
    /// Organization (tenant) identifier the user belongs to.
    pub organization_id: i32,
    /// Display name of the user.
    pub name: String,
    /// Unique username used for login and identity references.
    pub username: Option<String>,
    /// User email address.
    pub email: Option<String>,
    /// Optional avatar image URL.
    pub avatar_url: Option<String>,
    /// Role name (for example `admin` or `member`).
    pub role: String,
}

/// Task entity mapped from the `tasks` table and related projections.
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Task {
    /// Primary key of the task.
    pub id: i32,
    /// Organization (tenant) identifier the task belongs to.
    pub organization_id: i32,
    /// User identifier assigned to the task.
    pub member_id: i32,
    /// Human-readable task title.
    pub title: String,
    /// Current status label.
    pub status: String,
    /// Completion percentage in integer form.
    pub progress_rate: i32,
    /// Optional task tags aggregated from task-tag relations.
    #[sqlx(default)]
    pub tags: Option<Vec<String>>,
    /// Task creation timestamp (UTC).
    pub created_at: DateTime<Utc>,
    /// Aggregated total logged minutes for this task.
    #[sqlx(default)]
    pub total_duration_minutes: i64,
}

/// User-defined display group used to group members.
#[derive(Serialize, Deserialize, sqlx::FromRow, Clone, Debug)]
pub struct DisplayGroup {
    /// Primary key of the display group.
    pub id: i32,
    /// Organization (tenant) identifier.
    pub organization_id: i32,
    /// User who owns/created this display group.
    pub user_id: i32,
    /// Name of the display group.
    pub name: String,
    /// Member user IDs included in this group.
    #[sqlx(default)]
    pub member_ids: Vec<i32>,
    /// Group creation timestamp (UTC).
    pub created_at: DateTime<Utc>,
}

/// Time log record associated with a task and user.
#[derive(Serialize, Deserialize, sqlx::FromRow, Clone, Debug)]
pub struct TaskTimeLog {
    /// Primary key of the time log.
    pub id: i32,
    /// Organization (tenant) identifier.
    pub organization_id: i32,
    /// User who logged this work interval.
    pub user_id: i32,
    /// Related task identifier.
    pub task_id: i32,
    /// Start time of the logged interval.
    pub start_at: DateTime<Utc>,
    /// End time of the logged interval.
    pub end_at: DateTime<Utc>,
    /// Duration of the interval in minutes.
    pub duration_minutes: i64,
    /// Optional denormalized task title for reporting.
    #[sqlx(default)]
    pub task_title: Option<String>,
    /// Optional denormalized task status for reporting.
    #[sqlx(default)]
    pub task_status: Option<String>,
    /// Optional denormalized task progress for reporting.
    #[sqlx(default)]
    pub task_progress_rate: Option<i32>,
    /// Optional denormalized task tags for reporting.
    #[sqlx(default)]
    pub task_tags: Option<Vec<String>>,
    /// Aggregated total logged minutes for the related task.
    #[sqlx(default)]
    pub total_duration_minutes: i64,
}

/// Row projection for task reports that combines a task with user/timing stats.
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct TaskReportRow {
    /// Flattened task entity fields.
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub task: Task,
    /// Name of the related user/member.
    pub user_name: String,
    /// Total logged minutes included in this report row.
    pub total_duration_minutes: i64,
    /// Earliest logged start time in the selected period.
    pub start_at: Option<DateTime<Utc>>,
    /// Latest logged end time in the selected period.
    pub end_at: Option<DateTime<Utc>>,
}

/// Activity log entity representing auditable user actions.
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct ActivityLog {
    /// Primary key of the activity log entry.
    pub id: i32,
    /// Organization (tenant) identifier.
    pub organization_id: i32,
    /// User who performed the action.
    pub user_id: i32,
    /// Denormalized name of the acting user.
    pub user_name: String,
    /// Action identifier (for example `create`, `update`, `delete`).
    pub action: String,
    /// Target entity type affected by the action.
    pub target_type: String,
    /// Optional target entity ID.
    pub target_id: Option<i32>,
    /// Optional details payload for audit context.
    pub details: Option<String>,
    /// Timestamp when the action was recorded.
    pub created_at: DateTime<Utc>,
}

/// Daily report entity submitted by users.
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct DailyReport {
    /// Primary key of the daily report.
    pub id: i32,
    /// Organization (tenant) identifier.
    pub organization_id: i32,
    /// User who submitted the report.
    pub user_id: i32,
    /// Date that the report corresponds to.
    pub report_date: NaiveDate,
    /// Free-form report content.
    pub content: String,
    /// Timestamp when the report was created.
    pub created_at: DateTime<Utc>,
}

/// Invitation entity used for onboarding users into organizations.
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Invitation {
    /// Primary key of the invitation.
    pub id: i32,
    /// Organization that issued the invitation.
    pub organization_id: i32,
    /// Optional denormalized organization name.
    pub org_name: Option<String>,
    /// Invitation token used during the join flow.
    pub token: String,
    /// Role granted to the invited user.
    pub role: String,
    /// Expiration timestamp of the invitation.
    pub expires_at: DateTime<Utc>,
    /// Invitation creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// Notification entity delivered to users.
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Notification {
    /// Primary key of the notification.
    pub id: i32,
    /// Organization (tenant) identifier.
    pub organization_id: i32,
    /// Recipient user identifier.
    pub user_id: i32,
    /// Notification title.
    pub title: String,
    /// Optional notification body.
    pub body: Option<String>,
    /// Category name used to classify the notification.
    pub category: String,
    /// Optional related target type.
    pub target_type: Option<String>,
    /// Optional related target ID.
    pub target_id: Option<i32>,
    /// Read-state flag.
    pub is_read: bool,
    /// Notification creation timestamp.
    pub created_at: DateTime<Utc>,
}

// =============================
// Composite API Models
// =============================

/// User model enriched with attached time logs.
#[derive(Serialize)]
pub struct UserWithTimeLogs {
    /// Flattened user fields.
    #[serde(flatten)]
    pub user: User,
    /// Time logs for the selected context.
    pub time_logs: Vec<TaskTimeLog>,
}

/// Paginated wrapper for activity logs.
#[derive(Serialize)]
pub struct PaginatedLogs {
    /// Current page items.
    pub items: Vec<ActivityLog>,
    /// Total number of matching records.
    pub total: i64,
    /// Current page index (1-based or handler-defined convention).
    pub page: i64,
    /// Total number of pages for current pagination settings.
    pub total_pages: i64,
}

/// Paginated wrapper for notifications.
#[derive(Serialize)]
pub struct PaginatedNotifications {
    /// Current page items.
    pub items: Vec<Notification>,
    /// Total number of matching records.
    pub total: i64,
    /// Current page index (1-based or handler-defined convention).
    pub page: i64,
    /// Total number of pages for current pagination settings.
    pub total_pages: i64,
}

/// Top-level analytics payload for a user.
#[derive(Serialize, Deserialize)]
pub struct AnalyticsResponse {
    /// Display name of the analytics subject.
    pub user_name: String,
    /// Task-related aggregate statistics.
    pub task_stats: TaskStats,
    /// Report-related aggregate statistics.
    pub report_stats: ReportStats,
    /// Daily activity counts for heatmap rendering.
    pub heatmap: Vec<HeatmapDay>,
}

/// Aggregated task metrics used in analytics responses.
#[derive(Serialize, Deserialize)]
pub struct TaskStats {
    /// Lifetime completed task count.
    pub total_completed: i64,
    /// Number of tasks completed in the current week.
    pub completed_this_week: i64,
    /// Number of tasks completed in the previous week.
    pub completed_last_week: i64,
    /// Task counts grouped by status.
    pub by_status: Vec<StatusCount>,
}

/// Count tuple for a specific status label.
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct StatusCount {
    /// Status label.
    pub status: String,
    /// Number of items matching the status.
    pub count: i64,
}

/// Aggregated report metrics used in analytics responses.
#[derive(Serialize, Deserialize)]
pub struct ReportStats {
    /// Total number of submitted reports.
    pub total_submitted: i64,
}

/// Heatmap datapoint for a specific date.
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct HeatmapDay {
    /// Calendar day represented by this datapoint.
    pub date: NaiveDate,
    /// Number of events/reports/tasks counted on that day.
    pub count: i64,
}

/// JWT claims encoded into authentication tokens.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject identifier (username).
    pub sub: String,
    /// Authenticated user ID.
    pub user_id: i32,
    /// Organization (tenant) ID.
    pub organization_id: i32,
    /// User role embedded for authorization checks.
    pub role: String,
    /// Unix timestamp expiration.
    pub exp: usize,
}

/// Login response payload returned after successful authentication.
#[derive(Serialize)]
pub struct LoginResponse {
    /// Signed JWT access token.
    pub token: String,
    /// Authenticated user profile.
    pub user: User,
}

// =============================
// Request DTOs
// =============================

/// Request body for creating a display group.
#[derive(Deserialize)]
pub struct CreateDisplayGroupInput {
    /// Name of the new display group.
    pub name: String,
    /// User IDs that should belong to the group.
    pub member_ids: Vec<i32>,
}

/// Request body for login.
#[derive(Deserialize)]
pub struct LoginInput {
    /// Username used for authentication.
    pub username: String,
    /// Plaintext password provided by the client.
    pub password: String,
}

/// Request body for creating a user.
#[derive(Deserialize)]
pub struct CreateUserInput {
    /// Display name for the new user.
    pub name: String,
    /// Unique username for login.
    pub username: String,
    /// Plaintext password to hash and store.
    pub password: String,
    /// Optional avatar URL.
    pub avatar_url: Option<String>,
    /// Optional role override (defaults are handled by the handler/database).
    pub role: Option<String>,
}

/// Request body for creating a task.
#[derive(Deserialize)]
pub struct CreateTaskInput {
    /// User assigned to the task.
    pub member_id: i32,
    /// Task title.
    pub title: String,
    /// Optional tag list.
    pub tags: Option<Vec<String>>,
}

/// Request body for partially updating a task.
#[derive(Deserialize)]
pub struct UpdateTaskInput {
    /// Optional reassigned member ID.
    pub member_id: Option<i32>,
    /// Optional new title.
    pub title: Option<String>,
    /// Optional new status.
    pub status: Option<String>,
    /// Optional new progress percentage.
    pub progress_rate: Option<i32>,
    /// Optional replacement tags.
    pub tags: Option<Vec<String>>,
}

/// Request body for creating a task time log.
#[derive(Deserialize)]
pub struct AddTimeLogInput {
    /// User that logged this interval.
    pub user_id: i32,
    /// Optional existing task ID.
    pub task_id: Option<i32>,
    /// Optional title used when creating/associating task data.
    pub title: Option<String>,
    /// Optional tags used with task creation/association.
    pub tags: Option<Vec<String>>,
    /// Interval start timestamp.
    pub start_at: DateTime<Utc>,
    /// Interval end timestamp.
    pub end_at: DateTime<Utc>,
}

/// Request body for updating a task time log interval.
#[derive(Deserialize)]
pub struct UpdateTimeLogInput {
    /// Optional replacement start timestamp.
    pub start_at: Option<DateTime<Utc>>,
    /// Optional replacement end timestamp.
    pub end_at: Option<DateTime<Utc>>,
}

/// Request body for creating a daily report.
#[derive(Deserialize)]
pub struct CreateReportInput {
    /// Report date represented by the submission.
    pub report_date: NaiveDate,
    /// Report content/body.
    pub content: String,
}

/// Request body for organization registration.
#[derive(Deserialize)]
pub struct RegisterInput {
    /// Name of the organization to create.
    pub organization_name: String,
    /// Display name for the initial admin.
    pub admin_name: String,
    /// Username for the initial admin.
    pub username: String,
    /// Email for the initial admin.
    pub email: String,
    /// Plaintext password for the initial admin.
    pub password: String,
}

/// Request body for creating an invitation.
#[derive(Deserialize)]
pub struct CreateInvitationInput {
    /// Role granted to invited users.
    pub role: String,
}

/// Request body for accepting an invitation.
#[derive(Deserialize)]
pub struct JoinInput {
    /// Invitation token.
    pub token: String,
    /// New member display name.
    pub name: String,
    /// New member username.
    pub username: String,
    /// New member email.
    pub email: String,
    /// New member plaintext password.
    pub password: String,
}

/// Request body for initiating password reset.
#[derive(Deserialize)]
pub struct ForgotPasswordInput {
    /// Username of the account requesting reset.
    pub username: String,
}

/// Request body for completing password reset.
#[derive(Deserialize)]
pub struct ResetPasswordInput {
    /// Password reset token.
    pub token: String,
    /// Replacement plaintext password.
    pub new_password: String,
}

/// Request body for changing the authenticated user's password.
#[derive(Deserialize)]
pub struct UpdatePasswordInput {
    /// Current plaintext password for verification.
    pub current_password: String,
    /// New plaintext password to set.
    pub new_password: String,
}

/// Request body for updating a report's content.
#[derive(Deserialize)]
pub struct UpdateReportInput {
    /// Updated report content/body.
    pub content: String,
}

// =============================
// Query DTOs
// =============================

/// Query parameters for filtering report lists.
#[derive(Deserialize)]
pub struct ReportQuery {
    /// Optional date filter.
    pub date: Option<NaiveDate>,
    /// Optional user filter.
    pub user_id: Option<i32>,
}

/// Query parameters for listing users.
#[derive(Deserialize)]
pub struct GetUsersQuery {
    /// Optional date filter used by time-log/report aware user endpoints.
    pub date: Option<NaiveDate>,
}

/// Query parameters for task report endpoints.
#[derive(Deserialize)]
pub struct TaskReportQuery {
    /// Optional member/user filter.
    pub member_id: Option<i32>,
    /// Optional inclusive start date.
    pub start_date: Option<NaiveDate>,
    /// Optional inclusive end date.
    pub end_date: Option<NaiveDate>,
    /// Optional status CSV filter string.
    pub statuses: Option<String>,
}

/// Query parameters for activity log listing/export.
#[derive(Deserialize)]
pub struct LogQuery {
    /// Optional page number.
    pub page: Option<i64>,
    /// Optional page size.
    pub per_page: Option<i64>,
    /// Optional user filter.
    pub user_id: Option<i32>,
    /// Optional start date filter.
    pub start_date: Option<NaiveDate>,
    /// Optional end date filter.
    pub end_date: Option<NaiveDate>,
    /// Optional action filter.
    pub action: Option<String>,
    /// Optional target type filter.
    pub target_type: Option<String>,
}

/// Query parameters for task listing.
#[derive(Deserialize)]
pub struct GetTasksQuery {
    /// Optional member/user filter.
    pub member_id: Option<i32>,
    /// Optional status filter.
    pub status: Option<String>,
}

/// Query parameters for notification listing.
#[derive(Deserialize)]
pub struct NotificationQuery {
    /// Optional page number.
    pub page: Option<i64>,
    /// Optional page size.
    pub per_page: Option<i64>,
}
