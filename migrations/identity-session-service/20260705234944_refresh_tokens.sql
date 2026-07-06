-- Migration: refresh_tokens
-- Generated: 20260705234944

CREATE TABLE IF NOT EXISTS sesame_idam.refresh_tokens (
    id VARCHAR(32) PRIMARY KEY,
    type_field VARCHAR(16) NOT NULL,
    token TEXT NOT NULL,
    user_id UUID NOT NULL,
    session_id UUID,
    token_version INTEGER NOT NULL DEFAULT 0,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    issued_at TIMESTAMP WITH TIME ZONE NOT NULL,
    rotation_seq INTEGER NOT NULL DEFAULT 0,
    revoked BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);
