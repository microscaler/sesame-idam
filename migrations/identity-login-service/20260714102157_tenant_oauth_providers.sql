-- Migration: tenant_oauth_providers
-- Generated: 20260714102157

CREATE TABLE IF NOT EXISTS sesame_idam.tenant_oauth_providers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_slug VARCHAR(64) NOT NULL,
    provider VARCHAR(32) NOT NULL,
    client_id TEXT NOT NULL,
    redirect_uris TEXT NOT NULL,
    secret_env_key VARCHAR(255) NOT NULL,
    client_id_env_key VARCHAR(255),
    config_version INTEGER NOT NULL DEFAULT 0,
    last_rotated_at TIMESTAMP WITH TIME ZONE,
    last_rotated_by VARCHAR(255),
    enabled BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    UNIQUE(tenant_slug, provider)
);
