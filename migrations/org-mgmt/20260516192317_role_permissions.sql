-- Migration: role_permissions
-- Generated: 20260516192317

CREATE TABLE IF NOT EXISTS sesame_idam.role_permissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    role_id UUID NOT NULL REFERENCES sesame_idam.roles(id) ON DELETE CASCADE,
    permission_id UUID NOT NULL REFERENCES sesame_idam.permissions(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL
);
