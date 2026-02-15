-- Create members table
CREATE TABLE IF NOT EXISTS members (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) NOT NULL,
    avatar_url TEXT
);

-- Create tasks table
CREATE TABLE IF NOT EXISTS tasks (
    id SERIAL PRIMARY KEY,
    member_id INTEGER NOT NULL REFERENCES members(id) ON DELETE CASCADE,
    title VARCHAR(100) NOT NULL,
    status VARCHAR(20) DEFAULT 'todo', -- todo, doing, done
    progress_rate INTEGER DEFAULT 0,
    start_at TIMESTAMP WITH TIME ZONE NOT NULL,
    end_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);
