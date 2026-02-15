-- Add tags column to tasks table
ALTER TABLE tasks ADD COLUMN tags TEXT[] DEFAULT '{}';

-- Create activity_logs table
CREATE TABLE IF NOT EXISTS activity_logs (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    action VARCHAR(50) NOT NULL,
    target_type VARCHAR(20) NOT NULL, -- 'task', 'report', 'user'
    target_id INTEGER,
    details TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Index for faster log retrieval
CREATE INDEX idx_activity_logs_created_at ON activity_logs (created_at DESC);
