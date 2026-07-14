-- Reference policy: tenant isolation on sesame_idam.users (IDAM proof slice).
-- Canonical source: sql/rls/v1/reference-idam-users.sql

ALTER TABLE sesame_idam.users ENABLE ROW LEVEL SECURITY;
ALTER TABLE sesame_idam.users FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS sesame_users_tenant_isolation ON sesame_idam.users;

CREATE POLICY sesame_users_tenant_isolation ON sesame_idam.users
    FOR ALL
    USING (tenant_id = public.sesame_current_tenant_id())
    WITH CHECK (tenant_id = public.sesame_current_tenant_id());
