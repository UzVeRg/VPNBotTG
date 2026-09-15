CREATE TABLE orders (
    id BIGSERIAL PRIMARY KEY,

    telegram_id BIGINT NOT NULL
        REFERENCES users(telegram_id)
        ON DELETE RESTRICT,

    tariff_code TEXT NOT NULL,

    tariff_name TEXT NOT NULL,

    tariff_description TEXT NOT NULL,

    price_kopecks BIGINT NOT NULL
        CHECK (price_kopecks > 0),

    currency TEXT NOT NULL DEFAULT 'RUB'
        CHECK (currency = 'RUB'),

    duration_days INTEGER NOT NULL
        CHECK (duration_days > 0),

    traffic_gib BIGINT
        CHECK (traffic_gib IS NULL OR traffic_gib > 0),

    hwid_limit INTEGER NOT NULL
        CHECK (hwid_limit > 0),

    internal_squad_names TEXT[] NOT NULL,

    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN (
            'pending',
            'paid',
            'cancelled',
            'expired',
            'activation_failed'
        )),

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CHECK (cardinality(internal_squad_names) > 0)
);

CREATE INDEX orders_telegram_id_idx
    ON orders(telegram_id);

CREATE INDEX orders_status_idx
    ON orders(status);

CREATE INDEX orders_created_at_idx
    ON orders(created_at);