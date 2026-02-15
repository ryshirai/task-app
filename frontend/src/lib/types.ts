export type TaskStatus = 'todo' | 'doing' | 'done';
export type UserRole = 'admin' | 'user';

export interface Task {
    id: number;
    organization_id: number;
    member_id: number;
    title: string;
    status: TaskStatus;
    progress_rate: number;
    tags?: string[];
    start_at: string; // ISO 8601 string
    end_at: string;   // ISO 8601 string
    created_at: string; // ISO 8601 string
}

export interface User {
    id: number;
    organization_id: number;
    name: string;
    username: string;
    email?: string;
    avatar_url?: string;
    role: UserRole;
    tasks: Task[];
}

export interface ActivityLog {
    id: number;
    organization_id: number;
    user_id: number;
    user_name: string;
    action: string;
    target_type: string;
    target_id?: number;
    details?: string;
    created_at: string;
}

export interface DailyReport {
    id: number;
    organization_id: number;
    user_id: number;
    report_date: string; // YYYY-MM-DD
    content: string;
    created_at: string;
}

export interface Invitation {
    id: number;
    organization_id: number;
    org_name?: string | null;
    token: string;
    role: UserRole;
    expires_at: string;
    created_at: string;
}

export interface Notification {
    id: number;
    organization_id: number;
    user_id: number;
    title: string;
    body?: string | null;
    category: string;
    target_type?: string | null;
    target_id?: number | null;
    is_read: boolean;
    created_at: string;
}

export interface PaginatedNotifications {
    items: Notification[];
    total: number;
    page: number;
    total_pages: number;
}

export interface AuthState {
    token: string | null;
    user: User | null;
}

export interface StatusCount {
    status: string;
    count: number;
}

export interface HeatmapDay {
    date: string;
    count: number;
}

export interface TaskStats {
    total_completed: number;
    completed_this_week: number;
    completed_last_week: number;
    by_status: StatusCount[];
}

export interface ReportStats {
    total_submitted: number;
}

export interface PersonalAnalyticsResponse {
    user_name: string;
    task_stats: TaskStats;
    report_stats: ReportStats;
    heatmap: HeatmapDay[];
}
