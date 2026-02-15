-- Rename members to users
ALTER TABLE members RENAME TO users;

-- Add authentication and role columns to users
ALTER TABLE users ADD COLUMN username VARCHAR(50) UNIQUE;
ALTER TABLE users ADD COLUMN password_hash TEXT;
ALTER TABLE users ADD COLUMN role VARCHAR(20) DEFAULT 'user';

-- Update tasks table to reference users (renamed from members)
-- (PostgreSQL automatically updates the table name in the reference, but let's be explicit if needed or just rely on it)

-- Create daily_reports table
CREATE TABLE IF NOT EXISTS daily_reports (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    report_date DATE NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, report_date)
);

-- Seed an admin user (password: admin123)
-- Password hash for 'admin123' using argon2 (dummy for now, will be updated by code if needed, 
-- but for a migration, we should ideally have a valid hash or handle it in main.rs)
INSERT INTO users (name, username, password_hash, role) 
VALUES ('Administrator', 'admin', '$argon2id$v=19$m=19456,t=2,p=1$7v8x/vU6q3Q2N7Q2$hG8v8x/vU6q3Q2N7Q2N7Q2N7Q2N7Q2N7Q2N7Q2N7Q2', 'admin')
ON CONFLICT (username) DO NOTHING;
