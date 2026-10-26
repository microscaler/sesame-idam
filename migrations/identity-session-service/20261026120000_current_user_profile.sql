-- The current user's row and profile in one statement (26 Oct 2026).
--
-- /identity/me, OIDC userinfo and refresh's account check read the caller's users row and user_profiles row under
-- the tenant RLS policy. Through Lifeguard's contextual transaction that was five round trips (BEGIN,
-- rls_set_session, users, user_profiles, COMMIT) to the primary through PgBouncer and the data plane's Envoy, ~5 ms
-- each in pp, for ~0.1 ms of database work. This function does the same work in one round trip.
--
-- RLS is unchanged:
--   * SECURITY INVOKER: the caller's own privileges apply, and sesame_idam.users keeps FORCE ROW LEVEL SECURITY, so
--     the sesame_users_tenant_isolation policy filters the read exactly as before.
--   * The context is set by the same public.rls_set_session call with the same values Lifeguard's
--     with_pre_auth_tenant passes (tenant trimmed, nil subject and organisation, session 'pre-auth', no roles or
--     permissions), before the SELECT runs. A NULL tenant raises; a blank one leaves sesame_current_tenant_id() NULL
--     and matches no row (the service refuses a blank tenant before calling, as with_pre_auth_tenant did).
--   * rls_set_session sets transaction-local GUCs (set_config(..., true)). Called as one autocommit statement, the
--     implicit transaction ends with the statement and the context with it; through PgBouncer's transaction pooling
--     the server connection is never handed on with the context set. Called inside a caller's transaction, the
--     context lasts until that transaction ends, exactly as a direct rls_set_session call does.
--   * The tenant filter on users is kept in the query as well as in the policy.
-- Rows are returned as jsonb so the service rebuilds its existing models from them unchanged.

CREATE OR REPLACE FUNCTION sesame_idam.current_user_profile(p_tenant_id text, p_user_id uuid)
RETURNS TABLE (user_row jsonb, profile_row jsonb)
LANGUAGE plpgsql
VOLATILE
SECURITY INVOKER
SET search_path = pg_catalog
AS $$
BEGIN
    PERFORM public.rls_set_session(
        pg_catalog.btrim(p_tenant_id),
        '00000000-0000-0000-0000-000000000000'::uuid,
        '00000000-0000-0000-0000-000000000000'::uuid,
        'pre-auth',
        '[]'::jsonb,
        '[]'::jsonb,
        NULL,
        NULL
    );
    RETURN QUERY
        SELECT pg_catalog.to_jsonb(u), pg_catalog.to_jsonb(p)
          FROM sesame_idam.users AS u
          LEFT JOIN LATERAL (
                SELECT pr.*
                  FROM sesame_idam.user_profiles AS pr
                 WHERE pr.user_id = u.id
                 LIMIT 1
               ) AS p ON true
         WHERE u.tenant_id = p_tenant_id
           AND u.id = p_user_id;
END;
$$;

COMMENT ON FUNCTION sesame_idam.current_user_profile(text, uuid)
IS 'The caller''s users row and user_profiles row under tenant RLS in one statement (SECURITY INVOKER; same context as with_pre_auth_tenant)';

REVOKE ALL ON FUNCTION sesame_idam.current_user_profile(text, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION sesame_idam.current_user_profile(text, uuid) TO sesame_idam;
