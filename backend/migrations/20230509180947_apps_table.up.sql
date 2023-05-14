-- Add up migration script here
CREATE TABLE IF NOT EXISTS apps(
    id TEXT NOT NULL PRIMARY KEY,
    repository TEXT NOT NULL,
    is_verified BOOLEAN DEFAULT FALSE,
    last_submitted_version TEXT,
    first_seen TIMESTAMPTZ NULL,
    last_update TIMESTAMPTZ NULL
);
CREATE INDEX IF NOT EXISTS apps_id_repository_is_verified_first_seen_last_updated_indx ON apps (id, repository, is_verified, first_seen, last_update);