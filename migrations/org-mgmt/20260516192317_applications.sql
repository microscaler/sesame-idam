-- Migration: applications
-- Generated: 20260516192317

CREATE TABLE IF NOT EXISTS sesame_idam.applications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES sesame_idam.organizations(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    client_id VARCHAR(64) NOT NULL,
    client_secret TEXT,
    redirect_uris TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);
