-- Migration: api_keys
-- Generated: 20260516192317

CREATE TABLE IF NOT EXISTS sesame_idam.api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key_hash TEXT NOT NULL,
    key_prefix VARCHAR(16) NOT NULL,
    name VARCHAR(255) NOT NULL,
    tenant_id VARCHAR(255) NOT NULL,
    user_id UUID REFERENCES sesame_idam.users(id) ON DELETE CASCADE,
    org_id UUID REFERENCES sesame_idam.organizations(id) ON DELETE CASCADE,
    permissions TEXT,
    expires_at TIMESTAMP WITH TIME ZONE,
    active BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);
