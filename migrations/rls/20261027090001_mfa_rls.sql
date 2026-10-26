-- Tenant isolation on second factors, recovery codes and organisation sign-in policies (ADR-005). FORCE: the owner
-- group the services act as is bound by it too. identity-login-service reads and writes these under the tenant-only
-- context (with_pre_auth_tenant); org-mgmt writes policies under its validated session context.

ALTER TABLE sesame_idam.user_mfa_factors ENABLE ROW LEVEL SECURITY;
ALTER TABLE sesame_idam.user_mfa_factors FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS sesame_user_mfa_factors_tenant_isolation ON sesame_idam.user_mfa_factors;
CREATE POLICY sesame_user_mfa_factors_tenant_isolation ON sesame_idam.user_mfa_factors
    FOR ALL
    USING (tenant_id = public.sesame_current_tenant_id())
    WITH CHECK (tenant_id = public.sesame_current_tenant_id());

ALTER TABLE sesame_idam.user_recovery_codes ENABLE ROW LEVEL SECURITY;
ALTER TABLE sesame_idam.user_recovery_codes FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS sesame_user_recovery_codes_tenant_isolation ON sesame_idam.user_recovery_codes;
CREATE POLICY sesame_user_recovery_codes_tenant_isolation ON sesame_idam.user_recovery_codes
    FOR ALL
    USING (tenant_id = public.sesame_current_tenant_id())
    WITH CHECK (tenant_id = public.sesame_current_tenant_id());

ALTER TABLE sesame_idam.org_auth_policies ENABLE ROW LEVEL SECURITY;
ALTER TABLE sesame_idam.org_auth_policies FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS sesame_org_auth_policies_tenant_isolation ON sesame_idam.org_auth_policies;
CREATE POLICY sesame_org_auth_policies_tenant_isolation ON sesame_idam.org_auth_policies
    FOR ALL
    USING (tenant_id = public.sesame_current_tenant_id())
    WITH CHECK (tenant_id = public.sesame_current_tenant_id());
