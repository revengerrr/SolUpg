-- Phase 5: Fraud detection rules and risk scoring
CREATE TABLE IF NOT EXISTS fraud_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    rule_type VARCHAR(32) NOT NULL, -- 'velocity', 'threshold', 'sanctions', 'pattern', 'geo'
    config JSONB NOT NULL DEFAULT '{}',
    severity VARCHAR(10) NOT NULL DEFAULT 'warning', -- 'info', 'warning', 'critical', 'block'
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS risk_scores (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_address VARCHAR(64) NOT NULL,
    score INTEGER NOT NULL DEFAULT 0, -- 0-100, higher = riskier
    factors JSONB NOT NULL DEFAULT '[]',
    last_evaluated TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    source VARCHAR(32) NOT NULL DEFAULT 'internal', -- 'internal', 'chainalysis', 'trm_labs'
    UNIQUE (wallet_address, source)
);

CREATE TABLE IF NOT EXISTS fraud_alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rule_id UUID REFERENCES fraud_rules(id),
    wallet_address VARCHAR(64) NOT NULL,
    tx_signature VARCHAR(128),
    intent_id UUID,
    alert_type VARCHAR(32) NOT NULL,
    severity VARCHAR(10) NOT NULL,
    details JSONB NOT NULL DEFAULT '{}',
    status VARCHAR(20) NOT NULL DEFAULT 'open', -- 'open', 'reviewing', 'resolved', 'dismissed'
    resolved_by VARCHAR(100),
    resolved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS sanctions_list (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_address VARCHAR(64) NOT NULL UNIQUE,
    list_source VARCHAR(50) NOT NULL, -- 'OFAC', 'EU', 'UN', 'custom'
    reason TEXT,
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE INDEX idx_risk_scores_wallet ON risk_scores (wallet_address);
CREATE INDEX idx_risk_scores_score ON risk_scores (score) WHERE score >= 70;
CREATE INDEX idx_fraud_alerts_status ON fraud_alerts (status) WHERE status = 'open';
CREATE INDEX idx_fraud_alerts_wallet ON fraud_alerts (wallet_address);
CREATE INDEX idx_fraud_alerts_created ON fraud_alerts (created_at);
CREATE INDEX idx_sanctions_wallet ON sanctions_list (wallet_address) WHERE is_active;
