-- Phase 4: Reconciliation records
CREATE TABLE IF NOT EXISTS reconciliation_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_type VARCHAR(20) NOT NULL, -- 'streaming' or 'batch'
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,
    total_intents INTEGER NOT NULL DEFAULT 0,
    total_transactions INTEGER NOT NULL DEFAULT 0,
    matched INTEGER NOT NULL DEFAULT 0,
    mismatched INTEGER NOT NULL DEFAULT 0,
    orphaned_tx INTEGER NOT NULL DEFAULT 0,
    missing_tx INTEGER NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'running'
);

CREATE TABLE IF NOT EXISTS reconciliation_mismatches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES reconciliation_runs(id),
    intent_id UUID REFERENCES payment_intents(intent_id),
    tx_signature VARCHAR(128),
    mismatch_type VARCHAR(32) NOT NULL, -- 'amount', 'missing_tx', 'orphaned_tx', 'status'
    expected_value TEXT,
    actual_value TEXT,
    severity VARCHAR(10) NOT NULL DEFAULT 'warning', -- 'info', 'warning', 'critical'
    resolved BOOLEAN NOT NULL DEFAULT FALSE,
    resolved_at TIMESTAMPTZ,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_recon_runs_status ON reconciliation_runs (status);
CREATE INDEX idx_recon_mismatches_run ON reconciliation_mismatches (run_id);
CREATE INDEX idx_recon_mismatches_type ON reconciliation_mismatches (mismatch_type);
CREATE INDEX idx_recon_mismatches_unresolved ON reconciliation_mismatches (resolved) WHERE NOT resolved;
