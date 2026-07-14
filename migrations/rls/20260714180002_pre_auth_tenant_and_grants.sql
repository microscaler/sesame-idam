-- Pre-authentication tenant context + runtime grants for sesame_idam role.
-- Password login/register must set sesame.tenant_id before querying users under RLS.

CREATE OR REPLACE FUNCTION public.rls_set_pre_auth_tenant(p_tenant_id text)
RETURNS void
LANGUAGE plpgsql
SECURITY INVOKER
SET search_path = pg_catalog
AS $$
BEGIN
    IF p_tenant_id IS NULL
       OR NULLIF(pg_catalog.btrim(p_tenant_id), '') IS NULL THEN
        RAISE EXCEPTION 'pre-auth tenant id is required'
            USING ERRCODE = '22023';
    END IF;

    PERFORM pg_catalog.set_config('sesame.tenant_id', p_tenant_id, true);
END;
$$;

COMMENT ON FUNCTION public.rls_set_pre_auth_tenant(text)
IS 'Sesame RLS v1: set tenant-only context for unauthenticated login/register flows';

REVOKE ALL ON FUNCTION public.rls_set_pre_auth_tenant(text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION public.rls_set_pre_auth_tenant(text) TO sesame_idam;
GRANT EXECUTE ON FUNCTION public.rls_set_session(text, uuid, uuid, text, jsonb, jsonb, text, text) TO sesame_idam;
GRANT EXECUTE ON FUNCTION public.sesame_current_tenant_id() TO sesame_idam;
GRANT EXECUTE ON FUNCTION public.sesame_rls_contract_version() TO sesame_idam;
