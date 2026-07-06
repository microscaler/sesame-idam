-- Migration: saml_connections
-- Generated: 20260516192317

CREATE TABLE IF NOT EXISTS sesame_idam.saml_connections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES sesame_idam.organizations(id) ON DELETE CASCADE,
    issuer VARCHAR(255) NOT NULL,
    metadata_url TEXT,
    sso_url TEXT,
    signing_cert TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);
