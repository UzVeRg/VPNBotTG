CREATE TABLE users (
    telegram_id BIGINT PRIMARY KEY
        CHECK (telegram_id > 0),

    remnawave_user_id BIGINT UNIQUE,

    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    current_tariff_code TEXT,

    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN (
            'active',
            'blocked'
        )),

    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Переносим уже существующих участников trial,
-- чтобы добавление FK не сломалось на старых данных.
INSERT INTO users (
    telegram_id,
    remnawave_user_id,
    current_tariff_code
)
SELECT
    telegram_id,
    remnawave_user_id,
    CASE
        WHEN remnawave_user_id IS NOT NULL THEN 'trial'
        ELSE NULL
    END
FROM trial_claims
ON CONFLICT (telegram_id) DO NOTHING;

ALTER TABLE trial_claims
    ADD CONSTRAINT trial_claims_user_fk
    FOREIGN KEY (telegram_id)
    REFERENCES users(telegram_id)
    ON DELETE CASCADE;