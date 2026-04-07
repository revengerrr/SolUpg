CREATE TABLE IF NOT EXISTS payment_intents (
    intent_id UUID PRIMARY KEY,
    payer VARCHAR(64) NOT NULL,
    recipient_wallet VARCHAR(64) NOT NULL,
    source_mint VARCHAR(64) NOT NULL,
    destination_mint VARCHAR(64) NOT NULL,
    amount BIGINT NOT NULL,
    route_type VARCHAR(32) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    tx_signature VARCHAR(128),
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_intents_status ON payment_intents (status);
CREATE INDEX idx_intents_payer ON payment_intents (payer);
