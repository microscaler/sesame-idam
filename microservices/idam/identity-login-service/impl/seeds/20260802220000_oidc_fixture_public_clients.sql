-- Epic 14 / interactive PKCE fixtures: public clients with known redirects.
-- Used by conformance/oidc-v1 and live interactive BDD (not production apps).
--
-- Requires RLS tenant GUC when applied as sesame_idam:
--   BEGIN; SELECT set_config('app.tenant_id', 'acme', true); \i this_file.sql; COMMIT;

INSERT INTO sesame_idam.relying_party_clients
    (id, client_id, tenant_slug, portal, application_id, client_type,
     token_endpoint_auth_method, pkce_s256_required, authority_class,
     status, created_at, updated_at)
VALUES
    (
        'a1500002-0001-4000-8000-000000000001',
        'fixture-public-client',
        'acme',
        'frontend',
        'frontend',
        'public',
        'none',
        TRUE,
        'tenant',
        'active',
        NOW(),
        NOW()
    ),
    (
        'a1500002-0001-4000-8000-000000000002',
        'fixture-other-client',
        'acme',
        'frontend',
        'frontend',
        'public',
        'none',
        TRUE,
        'tenant',
        'active',
        NOW(),
        NOW()
    )
ON CONFLICT (client_id) DO UPDATE SET
    tenant_slug = EXCLUDED.tenant_slug,
    portal = EXCLUDED.portal,
    application_id = EXCLUDED.application_id,
    client_type = EXCLUDED.client_type,
    token_endpoint_auth_method = EXCLUDED.token_endpoint_auth_method,
    pkce_s256_required = EXCLUDED.pkce_s256_required,
    status = EXCLUDED.status,
    updated_at = NOW();

INSERT INTO sesame_idam.relying_party_client_redirect_uris
    (id, relying_party_client_id, kind, uri, created_at)
VALUES
    (
        'a1500002-0002-4000-8000-000000000001',
        'a1500002-0001-4000-8000-000000000001',
        'login',
        'https://client.example/callback',
        NOW()
    ),
    (
        'a1500002-0002-4000-8000-000000000002',
        'a1500002-0001-4000-8000-000000000002',
        'login',
        'https://client.example/callback',
        NOW()
    )
ON CONFLICT (relying_party_client_id, kind, uri) DO NOTHING;

INSERT INTO sesame_idam.relying_party_client_capabilities
    (id, relying_party_client_id, kind, value, created_at)
SELECT gen_random_uuid(), client_pk, kind, value, NOW()
FROM (
    VALUES
        ('a1500002-0001-4000-8000-000000000001'::uuid),
        ('a1500002-0001-4000-8000-000000000002'::uuid)
) AS clients(client_pk)
CROSS JOIN (
    VALUES
        ('grant', 'authorization_code'),
        ('grant', 'refresh_token'),
        ('response_type', 'code'),
        ('scope', 'openid'),
        ('scope', 'profile'),
        ('scope', 'email'),
        ('audience', 'sesame-idam')
) AS capability(kind, value)
ON CONFLICT (relying_party_client_id, kind, value) DO NOTHING;
