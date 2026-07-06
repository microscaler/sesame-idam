-- Migration: tokens
-- Generated: 20260516192317

CREATE TABLE IF NOT EXISTS sesame_idam.tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES sesame_idam.users(id) ON DELETE CASCADE,
    session_id UUID REFERENCES sesame_idam.sessions(id) ON DELETE CASCADE,
    type_field VARCHAR(32) NOT NULL,
    token TEXT NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);
