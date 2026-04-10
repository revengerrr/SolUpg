-- Phase 5: Immutable audit trail
CREATE TABLE IF NOT EXISTS audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type VARCHAR(50) NOT NULL, -- 'payment.created', 'escrow.released', 'merchant.registered', etc.
    actor_type VARCHAR(20) NOT NULL, -- 'system', 'merchant', 'admin', 'user'
    actor_id VARCHAR(100) NOT NULL,
    resource_type VARCHAR(30) NOT NULL, -- 'payment', 'escrow', 'merchant', 'alias', 'api_key'
    resource_id VARCHAR(128) NOT NULL,
    action VARCHAR(30) NOT NULL, -- 'create', 'update', 'delete', 'approve', 'reject', 'block'
    details JSONB NOT NULL DEFAULT '{}',
    ip_address VARCHAR(45),
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Partitioned index for time-range queries (regulatory exports)
CREATE INDEX idx_audit_log_created ON audit_log (created_at);
CREATE INDEX idx_audit_log_event ON audit_log (event_type);
CREATE INDEX idx_audit_log_actor ON audit_log (actor_type, actor_id);
CREATE INDEX idx_audit_log_resource ON audit_log (resource_type, resource_id);
