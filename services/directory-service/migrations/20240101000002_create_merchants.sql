CREATE TABLE IF NOT EXISTS merchants (
    id UUID PRIMARY KEY,
    merchant_id VARCHAR(64) NOT NULL,
    name VARCHAR(255) NOT NULL,
    wallet_address VARCHAR(64) NOT NULL,
    preferred_token VARCHAR(64),
    split_config VARCHAR(64),
    webhook_url VARCHAR(512),
    kyc_status VARCHAR(20) NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT merchants_merchant_id_key UNIQUE (merchant_id)
);

CREATE INDEX idx_merchants_merchant_id ON merchants (merchant_id);
