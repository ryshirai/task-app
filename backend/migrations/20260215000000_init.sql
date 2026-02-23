-- Clean up old schema
DROP TABLE IF EXISTS display_group_members;
DROP TABLE IF EXISTS display_groups;
DROP TABLE IF EXISTS invitations;
DROP TABLE IF EXISTS notifications;
DROP TABLE IF EXISTS activity_logs;
DROP TABLE IF EXISTS daily_reports;
DROP TABLE IF EXISTS task_tags;
DROP TABLE IF EXISTS task_time_logs;
DROP TABLE IF EXISTS tasks;
DROP TABLE IF EXISTS tags;
DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS organizations;

-- Organizations
CREATE TABLE organizations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Users & Auth
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    organization_id INTEGER NOT NULL DEFAULT 1, -- Single tenant for now, but ready for multi-tenant
    name TEXT NOT NULL,
    username TEXT NOT NULL,
    email TEXT,
    password_hash TEXT NOT NULL,
    avatar_url TEXT,
    role TEXT DEFAULT 'user', -- 'admin', 'user'
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(organization_id, username)
);

CREATE INDEX idx_users_org_username ON users(organization_id, username);

-- Tasks (Entity)
CREATE TABLE tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    organization_id INTEGER NOT NULL,
    member_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    status TEXT DEFAULT 'todo', -- 'todo', 'doing', 'done'
    progress_rate INTEGER DEFAULT 0 CHECK (progress_rate BETWEEN 0 AND 100),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_tasks_org_member ON tasks(organization_id, member_id);
CREATE INDEX idx_tasks_status ON tasks(organization_id, status);

-- Task Time Logs (Actual Work Time)
CREATE TABLE task_time_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    organization_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    start_at DATETIME NOT NULL,
    end_at DATETIME NOT NULL,
    duration_minutes INTEGER GENERATED ALWAYS AS (
        CAST(ROUND((julianday(end_at) - julianday(start_at)) * 1440) AS INTEGER)
    ) STORED,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    CHECK (end_at > start_at)
);

CREATE INDEX idx_time_logs_task ON task_time_logs(task_id);
CREATE INDEX idx_time_logs_user_date ON task_time_logs(user_id, start_at);
CREATE INDEX idx_time_logs_org_date ON task_time_logs(organization_id, start_at);

-- Tags (Normalized)
CREATE TABLE tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    organization_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(organization_id, name)
);

CREATE TABLE task_tags (
    task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (task_id, tag_id)
);

-- Daily Reports
CREATE TABLE daily_reports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    organization_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    report_date DATE NOT NULL,
    content TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(organization_id, user_id, report_date)
);

-- Activity Logs (Audit Trail)
CREATE TABLE activity_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    organization_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    action TEXT NOT NULL,
    target_type TEXT NOT NULL,
    target_id INTEGER,
    details TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_activity_logs_org_date ON activity_logs(organization_id, created_at DESC);

-- Notifications
CREATE TABLE notifications (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    organization_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    body TEXT,
    category TEXT NOT NULL, -- 'task_assigned', 'report_reminder', etc.
    target_type TEXT,
    target_id INTEGER,
    is_read BOOLEAN DEFAULT FALSE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_notifications_user_unread ON notifications(user_id, is_read) WHERE is_read = FALSE;

-- Invitations
CREATE TABLE invitations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    organization_id INTEGER NOT NULL,
    token TEXT NOT NULL UNIQUE,
    role TEXT NOT NULL DEFAULT 'user',
    expires_at DATETIME NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Display Groups (User Custom Filtering)
CREATE TABLE display_groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    organization_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE display_group_members (
    group_id INTEGER NOT NULL REFERENCES display_groups(id) ON DELETE CASCADE,
    member_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (group_id, member_id)
);

CREATE INDEX idx_display_groups_user ON display_groups(user_id);
