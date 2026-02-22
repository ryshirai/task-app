ALTER TABLE users ADD COLUMN email_verified BOOLEAN DEFAULT FALSE;
ALTER TABLE users ADD COLUMN email_verification_token VARCHAR(100);
CREATE INDEX idx_users_email_token ON users(email_verification_token);
