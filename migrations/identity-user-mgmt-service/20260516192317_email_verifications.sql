-- Migration: email_verifications
-- Generated: 20260516192317

CREATE TABLE IF NOT EXISTS sesame_idam.email_verifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES sesame_idam.users(id) ON DELETE CASCADE,
    token VARCHAR(64) NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);
