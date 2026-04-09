-- Phase 4: Transactions Ledger — indexed on-chain transactions
CREATE TABLE IF NOT EXISTS transactions_ledger (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tx_signature VARCHAR(128) NOT NULL UNIQUE,
    payment_id UUID REFERENCES payment_intents(intent_id),
    payer VARCHAR(64) NOT NULL,
    recipient VARCHAR(64) NOT NULL,
    amount BIGINT NOT NULL,
    token_mint VARCHAR(64) NOT NULL,
    fee_amount BIGINT NOT NULL DEFAULT 0,
    swap_source_token VARCHAR(64),
    swap_rate DOUBLE PRECISION,
    swap_slippage_bps INTEGER,
    status VARCHAR(20) NOT NULL DEFAULT 'confirmed',
    block_slot BIGINT NOT NULL,
    block_time TIMESTAMPTZ NOT NULL,
    program_id VARCHAR(64) NOT NULL,
    instruction_type VARCHAR(32) NOT NULL,
    raw_log TEXT,
    indexed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_txledger_payer ON transactions_ledger (payer);
CREATE INDEX idx_txledger_recipient ON transactions_ledger (recipient);
CREATE INDEX idx_txledger_payment_id ON transactions_ledger (payment_id);
CREATE INDEX idx_txledger_status ON transactions_ledger (status);
CREATE INDEX idx_txledger_block_time ON transactions_ledger (block_time);
CREATE INDEX idx_txledger_token_mint ON transactions_ledger (token_mint);
