-- Migration: social_accounts
-- Generated: 20260516192317

CREATE TABLE IF NOT EXISTS sesame_idam.social_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES sesame_idam.users(id) ON DELETE CASCADE,
    provider VARCHAR(64) NOT NULL,
    provider_user_id VARCHAR(255) NOT NULL,
    access_token TEXT,
    refresh_token TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);
