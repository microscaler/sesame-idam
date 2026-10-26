-- Second factors and passkeys (ADR-008): TOTP authenticators, WebAuthn passkeys and one-time recovery codes.
--
-- * user_mfa_factors: one row per enrolled authenticator. A TOTP row holds its secret sealed with AES-256-GCM under a
--   key derived from the deployment KEK (identity-login-service services/mfa/totp.rs; never stored or logged in the
--   clear) and the last accepted time step, so a code can be used once. A passkey row holds the webauthn-rs credential
--   (public key, counter, backup flags: no secret) and its credential id, unique within the tenant, for
--   username-less sign-in. `pending` rows are enrolments not yet confirmed with a first code; they never satisfy a
--   challenge and are replaced by the next enrolment.
-- * user_recovery_codes: SHA-256 of each one-time code (80 random bits each), never the code itself.
-- * The earlier mfa_setup table (identity-user-mgmt-service) is not used by these flows and
--   is left as it is.
--
-- Tenant isolation (ADR-005) is in rls/20261027090001_mfa_rls.sql. Re-runnable (every release applies every file).

CREATE TABLE IF NOT EXISTS sesame_idam.user_mfa_factors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id VARCHAR(255) NOT NULL,
    user_id UUID NOT NULL REFERENCES sesame_idam.users(id) ON DELETE CASCADE,
    kind VARCHAR(16) NOT NULL CHECK (kind IN ('totp', 'passkey')),
    status VARCHAR(16) NOT NULL CHECK (status IN ('pending', 'active')),
    label VARCHAR(64) NOT NULL,
    totp_secret_sealed TEXT,
    totp_last_step BIGINT,
    credential_id TEXT,
    passkey JSONB,
    passkey_synced BOOLEAN,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    confirmed_at TIMESTAMP WITH TIME ZONE,
    last_used_at TIMESTAMP WITH TIME ZONE,
    CHECK (kind <> 'totp' OR totp_secret_sealed IS NOT NULL),
    CHECK (kind <> 'passkey' OR (credential_id IS NOT NULL AND passkey IS NOT NULL))
);

CREATE INDEX IF NOT EXISTS ix_user_mfa_factors_user ON sesame_idam.user_mfa_factors (tenant_id, user_id);
-- A credential id identifies one passkey within a tenant (the browser returns it on username-less sign-in).
CREATE UNIQUE INDEX IF NOT EXISTS uq_user_mfa_factors_credential
    ON sesame_idam.user_mfa_factors (tenant_id, credential_id) WHERE credential_id IS NOT NULL;
-- At most one confirmed authenticator app per account: a second is a replacement, made by removing the first.
CREATE UNIQUE INDEX IF NOT EXISTS uq_user_mfa_factors_one_totp
    ON sesame_idam.user_mfa_factors (user_id) WHERE kind = 'totp' AND status = 'active';

CREATE TABLE IF NOT EXISTS sesame_idam.user_recovery_codes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id VARCHAR(255) NOT NULL,
    user_id UUID NOT NULL REFERENCES sesame_idam.users(id) ON DELETE CASCADE,
    code_hash TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    used_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX IF NOT EXISTS ix_user_recovery_codes_user ON sesame_idam.user_recovery_codes (tenant_id, user_id);
CREATE UNIQUE INDEX IF NOT EXISTS uq_user_recovery_codes_hash ON sesame_idam.user_recovery_codes (user_id, code_hash);
