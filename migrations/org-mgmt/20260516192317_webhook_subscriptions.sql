-- Migration: webhook_subscriptions
-- Generated: 20260516192317

CREATE TABLE IF NOT EXISTS sesame_idam.webhook_subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES sesame_idam.organizations(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    events TEXT NOT NULL,
    secret TEXT,
    active BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);
