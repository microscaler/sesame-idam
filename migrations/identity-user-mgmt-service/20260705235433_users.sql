-- Migration: users
-- Generated: 20260705235433

CREATE TABLE IF NOT EXISTS sesame_idam.users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL,
    password_hash TEXT NOT NULL,
    tenant_id VARCHAR(255) NOT NULL,
    status VARCHAR(32) NOT NULL,
    email_verified BOOLEAN NOT NULL DEFAULT false,
    phone VARCHAR(64),
    phone_verified BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    UNIQUE(tenant_id, email)
);
