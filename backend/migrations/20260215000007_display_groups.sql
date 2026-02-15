-- Create display_groups table
CREATE TABLE IF NOT EXISTS display_groups (
    id SERIAL PRIMARY KEY,
    organization_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create display_group_members table
CREATE TABLE IF NOT EXISTS display_group_members (
    group_id INTEGER NOT NULL REFERENCES display_groups(id) ON DELETE CASCADE,
    member_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (group_id, member_id)
);

CREATE INDEX idx_display_groups_user_id ON display_groups(user_id);
CREATE INDEX idx_display_groups_org_id ON display_groups(organization_id);
