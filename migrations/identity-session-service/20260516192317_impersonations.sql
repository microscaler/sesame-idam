-- Migration: impersonations
-- Generated: 20260516192317

CREATE TABLE IF NOT EXISTS sesame_idam.impersonations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES sesame_idam.users(id) ON DELETE CASCADE,
    impersonator_id UUID NOT NULL REFERENCES sesame_idam.users(id) ON DELETE CASCADE,
    session_id UUID NOT NULL REFERENCES sesame_idam.sessions(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    restored_at TIMESTAMP WITH TIME ZONE NOT NULL
);
