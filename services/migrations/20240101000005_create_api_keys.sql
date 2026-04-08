CREATE TABLE IF NOT EXISTS api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key_prefix VARCHAR(12) NOT NULL,
    key_hash VARCHAR(128) NOT NULL UNIQUE,
    merchant_id UUID REFERENCES merchants(id),
    name VARCHAR(128) NOT NULL,
    tier VARCHAR(20) NOT NULL DEFAULT 'free',
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_api_keys_hash ON api_keys (key_hash);
CREATE INDEX idx_api_keys_merchant ON api_keys (merchant_id);
