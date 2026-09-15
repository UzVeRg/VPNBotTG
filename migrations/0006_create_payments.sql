CREATE TABLE payments (
    id BIGSERIAL PRIMARY KEY,

    order_id BIGINT NOT NULL
        REFERENCES orders(id)
        ON DELETE RESTRICT,

    provider TEXT NOT NULL,

    provider_payment_id TEXT,

    idempotency_key TEXT NOT NULL UNIQUE,

    amount_kopecks BIGINT NOT NULL
        CHECK (amount_kopecks > 0),

    currency TEXT NOT NULL
        CHECK (currency = 'RUB'),

    status TEXT NOT NULL DEFAULT 'created'
        CHECK (status IN (
            'created',
            'pending',
            'paid',
            'failed',
            'cancelled',
            'expired'
        )),

    payment_url TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    paid_at TIMESTAMPTZ
);

CREATE INDEX payments_order_id_idx
    ON payments(order_id);

CREATE INDEX payments_status_idx
    ON payments(status);

CREATE UNIQUE INDEX payments_provider_payment_id_uidx
    ON payments(provider, provider_payment_id)
    WHERE provider_payment_id IS NOT NULL;