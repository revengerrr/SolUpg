-- Phase 4: Settlement batches for merchant reporting
CREATE TABLE IF NOT EXISTS settlement_batches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    merchant_id UUID NOT NULL,
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,
    total_transactions INTEGER NOT NULL DEFAULT 0,
    total_volume BIGINT NOT NULL DEFAULT 0,
    total_fees BIGINT NOT NULL DEFAULT 0,
    net_settlement BIGINT NOT NULL DEFAULT 0,
    currency_mint VARCHAR(64) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending', -- 'pending', 'confirmed', 'paid'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finalized_at TIMESTAMPTZ
);

-- Per-token breakdown within a settlement
CREATE TABLE IF NOT EXISTS settlement_token_breakdown (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    batch_id UUID NOT NULL REFERENCES settlement_batches(id),
    token_mint VARCHAR(64) NOT NULL,
    transaction_count INTEGER NOT NULL DEFAULT 0,
    volume BIGINT NOT NULL DEFAULT 0,
    fees BIGINT NOT NULL DEFAULT 0
);

CREATE INDEX idx_settlements_merchant ON settlement_batches (merchant_id);
CREATE INDEX idx_settlements_period ON settlement_batches (period_start, period_end);
CREATE INDEX idx_settlements_status ON settlement_batches (status);
CREATE INDEX idx_settlement_tokens_batch ON settlement_token_breakdown (batch_id);
