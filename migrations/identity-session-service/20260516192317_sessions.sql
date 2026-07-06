-- Migration: sessions
-- Generated: 20260516192317

CREATE TABLE IF NOT EXISTS sesame_idam.sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES sesame_idam.users(id) ON DELETE CASCADE,
    token TEXT NOT NULL,
    refresh_token TEXT NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    ip VARCHAR(64),
    user_agent TEXT,
    mfa_verified BOOLEAN NOT NULL DEFAULT false,
    impersonated_by UUID,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);
