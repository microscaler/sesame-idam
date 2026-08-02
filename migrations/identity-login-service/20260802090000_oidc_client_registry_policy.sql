-- Complete the standards-first OIDC relying-party registry (Epic 11).

ALTER TABLE sesame_idam.relying_party_clients
    ADD COLUMN IF NOT EXISTS application_id VARCHAR(64),
    ADD COLUMN IF NOT EXISTS token_endpoint_auth_method VARCHAR(32),
    ADD COLUMN IF NOT EXISTS pkce_s256_required BOOLEAN,
    ADD COLUMN IF NOT EXISTS authority_class VARCHAR(32);

UPDATE sesame_idam.relying_party_clients
SET application_id = COALESCE(application_id, portal),
    token_endpoint_auth_method = COALESCE(
        token_endpoint_auth_method,
        CASE WHEN client_type = 'public' THEN 'none' ELSE 'client_secret_basic' END
    ),
    pkce_s256_required = COALESCE(pkce_s256_required, client_type = 'public'),
    authority_class = COALESCE(authority_class, 'tenant');

ALTER TABLE sesame_idam.relying_party_clients
    ALTER COLUMN application_id SET NOT NULL,
    ALTER COLUMN token_endpoint_auth_method SET NOT NULL,
    ALTER COLUMN pkce_s256_required SET NOT NULL,
    ALTER COLUMN authority_class SET NOT NULL;

ALTER TABLE sesame_idam.relying_party_clients
    ADD CONSTRAINT relying_party_clients_type_check
        CHECK (client_type IN ('public', 'confidential')),
    ADD CONSTRAINT relying_party_clients_status_check
        CHECK (status IN ('active', 'disabled', 'deleted')),
    ADD CONSTRAINT relying_party_clients_auth_method_check
        CHECK (token_endpoint_auth_method IN ('none', 'client_secret_basic', 'client_secret_post')),
    ADD CONSTRAINT relying_party_clients_authority_check
        CHECK (authority_class IN ('tenant', 'platform')),
    ADD CONSTRAINT relying_party_clients_public_policy_check
        CHECK (
            client_type <> 'public'
            OR (
                token_endpoint_auth_method = 'none'
                AND pkce_s256_required
                AND authority_class = 'tenant'
            )
        );

CREATE TABLE IF NOT EXISTS sesame_idam.relying_party_client_redirect_uris (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    relying_party_client_id UUID NOT NULL
        REFERENCES sesame_idam.relying_party_clients(id) ON DELETE CASCADE,
    kind VARCHAR(32) NOT NULL CHECK (kind IN ('login', 'post_logout')),
    uri TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE (relying_party_client_id, kind, uri)
);

CREATE TABLE IF NOT EXISTS sesame_idam.relying_party_client_capabilities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    relying_party_client_id UUID NOT NULL
        REFERENCES sesame_idam.relying_party_clients(id) ON DELETE CASCADE,
    kind VARCHAR(32) NOT NULL
        CHECK (kind IN ('grant', 'response_type', 'scope', 'audience')),
    value VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE (relying_party_client_id, kind, value)
);

CREATE TABLE IF NOT EXISTS sesame_idam.relying_party_client_secrets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    relying_party_client_id UUID NOT NULL
        REFERENCES sesame_idam.relying_party_clients(id) ON DELETE CASCADE,
    secret_hash TEXT NOT NULL,
    status VARCHAR(32) NOT NULL CHECK (status IN ('active', 'revoked')),
    valid_from TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    valid_until TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMP WITH TIME ZONE,
    CHECK (valid_until IS NULL OR valid_until > valid_from)
);

CREATE INDEX IF NOT EXISTS idx_rp_redirect_client_kind
    ON sesame_idam.relying_party_client_redirect_uris (relying_party_client_id, kind);
CREATE INDEX IF NOT EXISTS idx_rp_capability_client_kind
    ON sesame_idam.relying_party_client_capabilities (relying_party_client_id, kind);
CREATE INDEX IF NOT EXISTS idx_rp_secret_client_status
    ON sesame_idam.relying_party_client_secrets (relying_party_client_id, status);
CREATE UNIQUE INDEX IF NOT EXISTS uq_rp_secret_current
    ON sesame_idam.relying_party_client_secrets (relying_party_client_id)
    WHERE status = 'active' AND valid_until IS NULL;

CREATE OR REPLACE FUNCTION sesame_idam.enforce_oidc_client_immutable_binding()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.client_type <> OLD.client_type
       OR NEW.authority_class <> OLD.authority_class
       OR NEW.tenant_slug <> OLD.tenant_slug
       OR NEW.application_id <> OLD.application_id THEN
        RAISE EXCEPTION 'OIDC client type, authority, tenant, and application are immutable'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS trg_oidc_client_immutable_binding
    ON sesame_idam.relying_party_clients;
CREATE TRIGGER trg_oidc_client_immutable_binding
    BEFORE UPDATE ON sesame_idam.relying_party_clients
    FOR EACH ROW EXECUTE FUNCTION sesame_idam.enforce_oidc_client_immutable_binding();

CREATE OR REPLACE FUNCTION sesame_idam.enforce_oidc_client_secret_policy()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM sesame_idam.relying_party_clients client
        WHERE client.id = NEW.relying_party_client_id
          AND client.client_type = 'confidential'
          AND client.status = 'active'
        FOR KEY SHARE
    ) THEN
        RAISE EXCEPTION 'Only active confidential OIDC clients can receive secrets'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS trg_oidc_client_secret_policy
    ON sesame_idam.relying_party_client_secrets;
CREATE TRIGGER trg_oidc_client_secret_policy
    BEFORE INSERT OR UPDATE ON sesame_idam.relying_party_client_secrets
    FOR EACH ROW
    WHEN (NEW.status = 'active')
    EXECUTE FUNCTION sesame_idam.enforce_oidc_client_secret_policy();

CREATE TABLE IF NOT EXISTS sesame_idam.oidc_application_migration_dispositions (
    application_id UUID PRIMARY KEY,
    disposition VARCHAR(32) NOT NULL
        CHECK (disposition IN ('migrated', 'manual_review', 'retired')),
    reason TEXT NOT NULL,
    recorded_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

DO $$
BEGIN
    IF to_regclass('sesame_idam.applications') IS NOT NULL THEN
        INSERT INTO sesame_idam.oidc_application_migration_dispositions
            (application_id, disposition, reason)
        SELECT id, 'manual_review',
               'Legacy application requires explicit client type, URI, grant, scope, audience, and secret policy'
        FROM sesame_idam.applications
        ON CONFLICT (application_id) DO NOTHING;
    END IF;
END
$$;

INSERT INTO sesame_idam.relying_party_client_capabilities
    (relying_party_client_id, kind, value)
SELECT id, capability.kind, capability.value
FROM sesame_idam.relying_party_clients
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

ALTER TABLE sesame_idam.relying_party_clients ENABLE ROW LEVEL SECURITY;
ALTER TABLE sesame_idam.relying_party_client_redirect_uris ENABLE ROW LEVEL SECURITY;
ALTER TABLE sesame_idam.relying_party_client_capabilities ENABLE ROW LEVEL SECURITY;
ALTER TABLE sesame_idam.relying_party_client_secrets ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS relying_party_clients_tenant_isolation
    ON sesame_idam.relying_party_clients;
CREATE POLICY relying_party_clients_tenant_isolation
    ON sesame_idam.relying_party_clients
    USING (tenant_slug = current_setting('app.tenant_id', true))
    WITH CHECK (
        tenant_slug = current_setting('app.tenant_id', true)
        AND authority_class = 'tenant'
    );

DROP POLICY IF EXISTS relying_party_redirects_tenant_isolation
    ON sesame_idam.relying_party_client_redirect_uris;
CREATE POLICY relying_party_redirects_tenant_isolation
    ON sesame_idam.relying_party_client_redirect_uris
    USING (EXISTS (
        SELECT 1
        FROM sesame_idam.relying_party_clients client
        WHERE client.id = relying_party_client_id
          AND client.tenant_slug = current_setting('app.tenant_id', true)
    ))
    WITH CHECK (EXISTS (
        SELECT 1
        FROM sesame_idam.relying_party_clients client
        WHERE client.id = relying_party_client_id
          AND client.tenant_slug = current_setting('app.tenant_id', true)
          AND client.authority_class = 'tenant'
    ));

DROP POLICY IF EXISTS relying_party_capabilities_tenant_isolation
    ON sesame_idam.relying_party_client_capabilities;
CREATE POLICY relying_party_capabilities_tenant_isolation
    ON sesame_idam.relying_party_client_capabilities
    USING (EXISTS (
        SELECT 1
        FROM sesame_idam.relying_party_clients client
        WHERE client.id = relying_party_client_id
          AND client.tenant_slug = current_setting('app.tenant_id', true)
    ))
    WITH CHECK (EXISTS (
        SELECT 1
        FROM sesame_idam.relying_party_clients client
        WHERE client.id = relying_party_client_id
          AND client.tenant_slug = current_setting('app.tenant_id', true)
          AND client.authority_class = 'tenant'
    ));

DROP POLICY IF EXISTS relying_party_secrets_tenant_isolation
    ON sesame_idam.relying_party_client_secrets;
CREATE POLICY relying_party_secrets_tenant_isolation
    ON sesame_idam.relying_party_client_secrets
    USING (EXISTS (
        SELECT 1
        FROM sesame_idam.relying_party_clients client
        WHERE client.id = relying_party_client_id
          AND client.tenant_slug = current_setting('app.tenant_id', true)
    ))
    WITH CHECK (EXISTS (
        SELECT 1
        FROM sesame_idam.relying_party_clients client
        WHERE client.id = relying_party_client_id
          AND client.tenant_slug = current_setting('app.tenant_id', true)
          AND client.authority_class = 'tenant'
    ));
