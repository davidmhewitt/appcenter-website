CREATE TABLE IF NOT EXISTS apps(
    id TEXT NOT NULL PRIMARY KEY,
    repository TEXT NOT NULL,
    is_verified BOOLEAN NOT NULL DEFAULT FALSE,
    last_submitted_version TEXT,
    first_seen TIMESTAMPTZ NULL,
    last_update TIMESTAMPTZ NULL
);

CREATE INDEX IF NOT EXISTS apps_is_verified ON apps (is_verified);

CREATE TABLE IF NOT EXISTS app_owners(
    user_id UUID NOT NULL,
    app_id TEXT NOT NULL,
    verified_owner BOOLEAN NOT NULL DEFAULT FALSE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (app_id) REFERENCES apps(id) ON DELETE CASCADE,
    PRIMARY KEY(user_id, app_id)
);

CREATE INDEX IF NOT EXISTS app_owners_verified_owner ON app_owners (verified_owner);