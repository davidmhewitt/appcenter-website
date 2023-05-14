CREATE TABLE IF NOT EXISTS app_owners(
    user_id UUID NOT NULL,
    app_id TEXT NOT NULL,
    verified_owner BOOLEAN NOT NULL DEFAULT FALSE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (app_id) REFERENCES apps(id) ON DELETE CASCADE,
    PRIMARY KEY(user_id, app_id)
);
CREATE INDEX IF NOT EXISTS app_owners_user_id_app_id_owner_indx ON app_owners (user_id, app_id, verified_owner);