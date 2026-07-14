-- Executable reference policy for a production-shaped Hauliage resource.
-- This schema is a contract fixture, not a Hauliage application migration.
-- Consumers should adapt table and permission names while preserving both
-- USING and WITH CHECK boundaries.

CREATE SCHEMA IF NOT EXISTS sesame_rls_reference;

CREATE TABLE IF NOT EXISTS sesame_rls_reference.hauliage_consignments (
    id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL,
    organization_id uuid NOT NULL,
    reference text NOT NULL,
    payload jsonb NOT NULL DEFAULT '{}'::jsonb
);

ALTER TABLE sesame_rls_reference.hauliage_consignments ENABLE ROW LEVEL SECURITY;
ALTER TABLE sesame_rls_reference.hauliage_consignments FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS sesame_consignments_select
    ON sesame_rls_reference.hauliage_consignments;
CREATE POLICY sesame_consignments_select
    ON sesame_rls_reference.hauliage_consignments
    FOR SELECT
    USING (
        tenant_id = public.sesame_current_tenant_id()
        AND organization_id = public.sesame_current_organization_id()
        AND (
            public.sesame_has_role('admin')
            OR public.sesame_has_permission('consignments:read')
        )
    );

DROP POLICY IF EXISTS sesame_consignments_insert
    ON sesame_rls_reference.hauliage_consignments;
CREATE POLICY sesame_consignments_insert
    ON sesame_rls_reference.hauliage_consignments
    FOR INSERT
    WITH CHECK (
        tenant_id = public.sesame_current_tenant_id()
        AND organization_id = public.sesame_current_organization_id()
        AND (
            public.sesame_has_role('admin')
            OR public.sesame_has_permission('consignments:write')
        )
    );

DROP POLICY IF EXISTS sesame_consignments_update
    ON sesame_rls_reference.hauliage_consignments;
CREATE POLICY sesame_consignments_update
    ON sesame_rls_reference.hauliage_consignments
    FOR UPDATE
    USING (
        tenant_id = public.sesame_current_tenant_id()
        AND organization_id = public.sesame_current_organization_id()
    )
    WITH CHECK (
        tenant_id = public.sesame_current_tenant_id()
        AND organization_id = public.sesame_current_organization_id()
        AND (
            public.sesame_has_role('admin')
            OR public.sesame_has_permission('consignments:write')
        )
    );

DROP POLICY IF EXISTS sesame_consignments_delete
    ON sesame_rls_reference.hauliage_consignments;
CREATE POLICY sesame_consignments_delete
    ON sesame_rls_reference.hauliage_consignments
    FOR DELETE
    USING (
        tenant_id = public.sesame_current_tenant_id()
        AND organization_id = public.sesame_current_organization_id()
        AND (
            public.sesame_has_role('admin')
            OR public.sesame_has_permission('consignments:delete')
        )
    );

REVOKE ALL ON SCHEMA sesame_rls_reference FROM PUBLIC;
REVOKE ALL ON TABLE sesame_rls_reference.hauliage_consignments FROM PUBLIC;
