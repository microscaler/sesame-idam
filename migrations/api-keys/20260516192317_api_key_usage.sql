-- Migration: api_key_usage
-- Generated: 20260516192317

CREATE TABLE IF NOT EXISTS sesame_idam.api_key_usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key_id UUID NOT NULL REFERENCES sesame_idam.api_keys(id) ON DELETE CASCADE,
    endpoint VARCHAR(255) NOT NULL,
    method VARCHAR(16) NOT NULL,
    tenant_id VARCHAR(255) NOT NULL,
    ip VARCHAR(64) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL
);
