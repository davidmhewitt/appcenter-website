ALTER TABLE apps
ADD COLUMN is_published BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX IF NOT EXISTS apps_is_published ON apps (is_published);