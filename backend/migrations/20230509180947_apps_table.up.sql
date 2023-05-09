-- Add up migration script here
CREATE TABLE IF NOT EXISTS apps(
    id TEXT NOT NULL PRIMARY KEY,
    user_id UUID NOT NULL UNIQUE,
    repository TEXT NOT NULL,
    is_verified BOOLEAN DEFAULT FALSE,
    last_submitted_version TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS apps_id_repository_last_version_indx ON apps (id, repository, last_submitted_version);