-- Migration: magic_link_tokens
-- Generated: 20260516192317

CREATE TABLE IF NOT EXISTS sesame_idam.magic_link_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES sesame_idam.users(id) ON DELETE CASCADE,
    link TEXT NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    used BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);
