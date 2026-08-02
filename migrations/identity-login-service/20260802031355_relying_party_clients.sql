-- Migration: relying_party_clients
-- Generated: 20260802031355

CREATE TABLE IF NOT EXISTS sesame_idam.relying_party_clients (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_id VARCHAR(128) NOT NULL UNIQUE,
    tenant_slug VARCHAR(64) NOT NULL REFERENCES sesame_idam.tenants(slug) ON DELETE CASCADE,
    portal VARCHAR(64) NOT NULL,
    client_type VARCHAR(32) NOT NULL,
    status VARCHAR(32) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);
