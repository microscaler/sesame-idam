-- Migration: audit_events
-- Generated: 20260516192317

CREATE TABLE IF NOT EXISTS sesame_idam.audit_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id VARCHAR(255) NOT NULL,
    user_id UUID REFERENCES sesame_idam.users(id) ON DELETE SET NULL,
    event_type VARCHAR(64) NOT NULL,
    severity VARCHAR(32) NOT NULL,
    actor VARCHAR(32) NOT NULL,
    data TEXT,
    ip VARCHAR(64),
    user_agent VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL
);
