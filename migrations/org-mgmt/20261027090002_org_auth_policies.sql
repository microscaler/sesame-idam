-- Organisation sign-in policy (ADR-008 §2, minimal): an organisation's admins can require every member to sign in
-- with a second factor ('mfa': TOTP, passkey or a recovery code) or with a passkey ('passkey', phishing-resistant).
-- identity-login-service reads it at sign-in: a member without the required method is made to enrol one before any
-- token is issued, and a member who has one must use it. The strictest policy of the user's active memberships
-- applies. No row: 'none' (the tenant default; retail users are not affected).
--
-- Tenant isolation in rls/20261027090001_mfa_rls.sql. Re-runnable.

CREATE TABLE IF NOT EXISTS sesame_idam.org_auth_policies (
    org_id UUID PRIMARY KEY REFERENCES sesame_idam.organizations(id) ON DELETE CASCADE,
    tenant_id VARCHAR(255) NOT NULL,
    require_method VARCHAR(16) NOT NULL CHECK (require_method IN ('none', 'mfa', 'passkey')),
    updated_by UUID,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS ix_org_auth_policies_tenant ON sesame_idam.org_auth_policies (tenant_id);
