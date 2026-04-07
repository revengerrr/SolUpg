CREATE TABLE IF NOT EXISTS aliases (
    id UUID PRIMARY KEY,
    alias_type VARCHAR(20) NOT NULL,
    alias_value VARCHAR(255) NOT NULL,
    wallet_address VARCHAR(64) NOT NULL,
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT aliases_alias_type_alias_value_key UNIQUE (alias_type, alias_value)
);

CREATE INDEX idx_aliases_value ON aliases (alias_value);
CREATE INDEX idx_aliases_wallet ON aliases (wallet_address);
