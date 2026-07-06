-- Migration: audit_retention_policies
-- Generated: 20260516192317

CREATE TABLE IF NOT EXISTS sesame_idam.audit_retention_policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id VARCHAR(255) NOT NULL,
    retention_days INTEGER NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);
