-- Migration: tenant_sms_config
-- Generated: 20260724223319

CREATE TABLE IF NOT EXISTS sesame_idam.tenant_sms_config (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id VARCHAR(64) NOT NULL,
    environment VARCHAR(32) NOT NULL,
    provider VARCHAR(32) NOT NULL,
    custody_mode VARCHAR(16) NOT NULL,
    connected_account_sid VARCHAR(64),
    account_sid VARCHAR(64),
    auth_token_ciphertext TEXT,
    auth_token_nonce TEXT,
    dek_wrapped TEXT,
    messaging_service_sid VARCHAR(64),
    from_number VARCHAR(32),
    campaign_ref VARCHAR(64),
    daily_spend_ceiling_cents INTEGER NOT NULL DEFAULT 0,
    status VARCHAR(32) NOT NULL,
    last_validated_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    UNIQUE(tenant_id, environment)
);
