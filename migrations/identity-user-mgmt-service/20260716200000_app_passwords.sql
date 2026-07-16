-- App passwords / protocol credentials (PRD-OPENGROUPWARE F2).
--
-- Per-user static credentials for protocol clients that cannot do OIDC
-- redirects (IMAP/SMTP/DAV today; any non-browser client of a tenant
-- product in general). argon2id PHC hashes only — plaintext is shown once
-- at issue time by the API and never stored. Verified by trusted relying
-- parties through the rp_directory bridge (F3) or a future verify endpoint.

CREATE TABLE IF NOT EXISTS sesame_idam.app_passwords (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    VARCHAR(255) NOT NULL,
    user_id      UUID NOT NULL REFERENCES sesame_idam.users(id) ON DELETE CASCADE,
    label        VARCHAR(255) NOT NULL,
    secret_phc   TEXT NOT NULL,
    scopes       TEXT[] NOT NULL DEFAULT '{mail}',
    created_at   TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    last_used_at TIMESTAMP WITH TIME ZONE,
    revoked_at   TIMESTAMP WITH TIME ZONE
);

CREATE INDEX IF NOT EXISTS app_passwords_user_idx
    ON sesame_idam.app_passwords (user_id) WHERE revoked_at IS NULL;
CREATE INDEX IF NOT EXISTS app_passwords_tenant_idx
    ON sesame_idam.app_passwords (tenant_id);

-- Tenant isolation, same contract as sesame_idam.users.
ALTER TABLE sesame_idam.app_passwords ENABLE ROW LEVEL SECURITY;
ALTER TABLE sesame_idam.app_passwords FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS sesame_app_passwords_tenant_isolation
    ON sesame_idam.app_passwords;

CREATE POLICY sesame_app_passwords_tenant_isolation
    ON sesame_idam.app_passwords
    FOR ALL
    USING (tenant_id = public.sesame_current_tenant_id())
    WITH CHECK (tenant_id = public.sesame_current_tenant_id());
