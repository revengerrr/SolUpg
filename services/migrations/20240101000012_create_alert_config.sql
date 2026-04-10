-- Phase 5: Alert configuration and notification tracking
CREATE TABLE IF NOT EXISTS alert_channels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    channel_type VARCHAR(20) NOT NULL, -- 'slack', 'pagerduty', 'email', 'webhook'
    config JSONB NOT NULL DEFAULT '{}', -- webhook_url, api_key, email, etc.
    severity_filter VARCHAR(10)[] NOT NULL DEFAULT ARRAY['critical'],
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS alert_notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alert_id UUID NOT NULL REFERENCES fraud_alerts(id),
    channel_id UUID NOT NULL REFERENCES alert_channels(id),
    status VARCHAR(20) NOT NULL DEFAULT 'pending', -- 'pending', 'sent', 'failed'
    attempts INTEGER NOT NULL DEFAULT 0,
    last_attempt TIMESTAMPTZ,
    sent_at TIMESTAMPTZ,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_alert_notif_status ON alert_notifications (status) WHERE status = 'pending';
CREATE INDEX idx_alert_notif_alert ON alert_notifications (alert_id);
