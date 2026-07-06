-- Migration: mfa_setup
-- Generated: 20260516192317

CREATE TABLE IF NOT EXISTS sesame_idam.mfa_setup (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES sesame_idam.users(id) ON DELETE CASCADE,
    factor_type VARCHAR(32) NOT NULL,
    secret TEXT,
    enabled BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);
