CREATE TABLE IF NOT EXISTS github_auth(
    user_id UUID PRIMARY KEY REFERENCES users ON DELETE CASCADE,
    github_user_id TEXT NULL,
    github_access_token TEXT NULL,
    github_refresh_token TEXT NULL
);