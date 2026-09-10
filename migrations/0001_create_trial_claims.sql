CREATE TABLE trial_claims (
    telegram_id BIGINT PRIMARY KEY
        CHECK (telegram_id > 0),

    status TEXT NOT NULL
        CHECK (status IN (
            'creating',
            'active',
            'failed',
            'ineligible'
        )),

    remnawave_user_id BIGINT UNIQUE,

    issued_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_trial_claims_status
    ON trial_claims(status);