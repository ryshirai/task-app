-- 1. 一時テーブルの作成
CREATE TABLE task_time_logs_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    organization_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    task_id INTEGER NOT NULL,
    start_at TEXT NOT NULL,
    end_at TEXT NOT NULL,
    duration_minutes INTEGER GENERATED ALWAYS AS (
        CAST(ROUND((julianday(end_at) - julianday(start_at)) * 1440) AS INTEGER)
    ) STORED,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (end_at > start_at),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

-- 2. データの移行
INSERT INTO task_time_logs_new (id, organization_id, user_id, task_id, start_at, end_at, created_at)
SELECT id, organization_id, user_id, task_id, start_at, end_at, created_at FROM task_time_logs;

-- 3. 旧テーブルの削除とリネーム
DROP TABLE task_time_logs;
ALTER TABLE task_time_logs_new RENAME TO task_time_logs;

-- 4. インデックスの再作成
CREATE INDEX idx_time_logs_task ON task_time_logs (task_id);
CREATE INDEX idx_time_logs_user_date ON task_time_logs (user_id, start_at);
CREATE INDEX idx_time_logs_org_date ON task_time_logs (organization_id, start_at);
