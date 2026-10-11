-- Tenant isolation on organisation domain claims and their history (ADR-005; story 17.3). FORCE: the owner group the
-- services act as is bound by it too. Events: read and append only (no UPDATE or DELETE policy).

ALTER TABLE sesame_idam.org_domains ENABLE ROW LEVEL SECURITY;
ALTER TABLE sesame_idam.org_domains FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS sesame_org_domains_tenant_isolation ON sesame_idam.org_domains;
CREATE POLICY sesame_org_domains_tenant_isolation ON sesame_idam.org_domains
    FOR ALL
    USING (tenant_id = public.sesame_current_tenant_id())
    WITH CHECK (tenant_id = public.sesame_current_tenant_id());

ALTER TABLE sesame_idam.org_domain_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE sesame_idam.org_domain_events FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS sesame_org_domain_events_read ON sesame_idam.org_domain_events;
DROP POLICY IF EXISTS sesame_org_domain_events_append ON sesame_idam.org_domain_events;
CREATE POLICY sesame_org_domain_events_read ON sesame_idam.org_domain_events
    FOR SELECT USING (tenant_id = public.sesame_current_tenant_id());
CREATE POLICY sesame_org_domain_events_append ON sesame_idam.org_domain_events
    FOR INSERT WITH CHECK (tenant_id = public.sesame_current_tenant_id());
