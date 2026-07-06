-- Migration: archived_api_keys
-- Generated: 20260516192317

CREATE TABLE IF NOT EXISTS sesame_idam.archived_api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key_hash TEXT NOT NULL,
    key_prefix VARCHAR(16) NOT NULL,
    name VARCHAR(255) NOT NULL,
    reason TEXT NOT NULL,
    archived_at TIMESTAMP WITH TIME ZONE NOT NULL
);
