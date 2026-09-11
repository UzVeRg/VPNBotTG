CREATE TABLE tariffs (
    code TEXT PRIMARY KEY,

    name TEXT NOT NULL,

    price_kopecks BIGINT NOT NULL
        CHECK (price_kopecks >= 0),

    duration_days INTEGER NOT NULL
        CHECK (duration_days > 0),

    traffic_gib BIGINT
        CHECK (traffic_gib IS NULL OR traffic_gib > 0),

    hwid_limit INTEGER NOT NULL
        CHECK (hwid_limit > 0),

    internal_squad_names TEXT[] NOT NULL,

    is_active BOOLEAN NOT NULL DEFAULT TRUE,

    sort_order INTEGER NOT NULL DEFAULT 0,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CHECK (cardinality(internal_squad_names) > 0)
);