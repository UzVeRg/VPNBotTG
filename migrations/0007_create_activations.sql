CREATE TABLE activations (
    id BIGSERIAL PRIMARY KEY,

    order_id BIGINT NOT NULL UNIQUE
        REFERENCES orders(id)
        ON DELETE RESTRICT,

    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN (
            'pending',
            'processing',
            'succeeded',
            'failed'
        )),

    operation TEXT
        CHECK (
            operation IS NULL
            OR operation IN ('create', 'update')
        ),

    remnawave_user_id BIGINT,

    target_expire_at TIMESTAMPTZ,

    target_traffic_limit_bytes BIGINT
        CHECK (
            target_traffic_limit_bytes IS NULL
            OR target_traffic_limit_bytes >= 0
        ),

    target_hwid_limit INTEGER
        CHECK (
            target_hwid_limit IS NULL
            OR target_hwid_limit > 0
        ),

    target_internal_squad_uuids TEXT[],

    last_error TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    started_at TIMESTAMPTZ,

    completed_at TIMESTAMPTZ,

    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX activations_status_idx
    ON activations(status);

CREATE INDEX activations_remnawave_user_id_idx
    ON activations(remnawave_user_id);