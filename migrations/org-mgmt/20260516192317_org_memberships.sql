-- Migration: org_memberships
-- Generated: 20260516192317

CREATE TABLE IF NOT EXISTS sesame_idam.org_memberships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES sesame_idam.organizations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES sesame_idam.users(id) ON DELETE CASCADE,
    role VARCHAR(255) NOT NULL,
    status VARCHAR(32) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);
