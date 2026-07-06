-- Migration: role_assignments
-- Generated: 20260516192317

CREATE TABLE IF NOT EXISTS sesame_idam.role_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    principal_id UUID NOT NULL REFERENCES sesame_idam.users(id) ON DELETE CASCADE,
    role_name VARCHAR(255) NOT NULL,
    resource_type VARCHAR(255) NOT NULL,
    resource_id UUID REFERENCES sesame_idam.organizations(id) ON DELETE CASCADE,
    tenant_id VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);
