-- Create organizations table
CREATE TABLE IF NOT EXISTS organizations (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create a default organization for existing data
INSERT INTO organizations (name) VALUES ('Default Organization');

-- Add organization_id to users
ALTER TABLE users ADD COLUMN organization_id INTEGER REFERENCES organizations(id);
UPDATE users SET organization_id = (SELECT id FROM organizations LIMIT 1);
ALTER TABLE users ALTER COLUMN organization_id SET NOT NULL;

-- Add email to users (for invitations and resets)
ALTER TABLE users ADD COLUMN email VARCHAR(255) UNIQUE;

-- Create invitations table
CREATE TABLE IF NOT EXISTS invitations (
    id SERIAL PRIMARY KEY,
    organization_id INTEGER NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    token VARCHAR(64) UNIQUE NOT NULL,
    role VARCHAR(20) DEFAULT 'user',
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create password_resets table
CREATE TABLE IF NOT EXISTS password_resets (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token VARCHAR(64) UNIQUE NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Optional: Add organization_id directly to major tables for better isolation/performance
ALTER TABLE tasks ADD COLUMN organization_id INTEGER REFERENCES organizations(id);
UPDATE tasks SET organization_id = (SELECT organization_id FROM users WHERE users.id = tasks.member_id);
-- If some tasks don't have users? No, member_id is NOT NULL.
ALTER TABLE tasks ALTER COLUMN organization_id SET NOT NULL;

ALTER TABLE daily_reports ADD COLUMN organization_id INTEGER REFERENCES organizations(id);
UPDATE daily_reports SET organization_id = (SELECT organization_id FROM users WHERE users.id = daily_reports.user_id);
ALTER TABLE daily_reports ALTER COLUMN organization_id SET NOT NULL;

ALTER TABLE activity_logs ADD COLUMN organization_id INTEGER REFERENCES organizations(id);
UPDATE activity_logs SET organization_id = (SELECT organization_id FROM users WHERE users.id = activity_logs.user_id);
ALTER TABLE activity_logs ALTER COLUMN organization_id SET NOT NULL;
