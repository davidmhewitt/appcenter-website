CREATE TABLE IF NOT EXISTS stripe_accounts(
    user_id UUID PRIMARY KEY REFERENCES users ON DELETE CASCADE,
    stripe_account_id TEXT NOT NULL
);