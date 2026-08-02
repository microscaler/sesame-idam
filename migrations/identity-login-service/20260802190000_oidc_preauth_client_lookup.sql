-- Pre-auth OIDC client resolution (Epic 12).
--
-- Authorization and token endpoints derive tenant from client_id before any
-- app.tenant_id GUC exists. Tenant-isolation policies alone make those lookups
-- return zero rows and surface as invalid_client.
--
-- Add SELECT-only policies for the unscoped (pre-auth) path. Mutations remain
-- tenant-bound via the existing WITH CHECK policies.

DROP POLICY IF EXISTS relying_party_clients_preauth_select
    ON sesame_idam.relying_party_clients;
CREATE POLICY relying_party_clients_preauth_select
    ON sesame_idam.relying_party_clients
    FOR SELECT
    USING (
        status = 'active'
        AND COALESCE(NULLIF(current_setting('app.tenant_id', true), ''), '') = ''
    );

DROP POLICY IF EXISTS relying_party_redirects_preauth_select
    ON sesame_idam.relying_party_client_redirect_uris;
CREATE POLICY relying_party_redirects_preauth_select
    ON sesame_idam.relying_party_client_redirect_uris
    FOR SELECT
    USING (
        COALESCE(NULLIF(current_setting('app.tenant_id', true), ''), '') = ''
        AND EXISTS (
            SELECT 1
            FROM sesame_idam.relying_party_clients client
            WHERE client.id = relying_party_client_id
              AND client.status = 'active'
        )
    );

DROP POLICY IF EXISTS relying_party_capabilities_preauth_select
    ON sesame_idam.relying_party_client_capabilities;
CREATE POLICY relying_party_capabilities_preauth_select
    ON sesame_idam.relying_party_client_capabilities
    FOR SELECT
    USING (
        COALESCE(NULLIF(current_setting('app.tenant_id', true), ''), '') = ''
        AND EXISTS (
            SELECT 1
            FROM sesame_idam.relying_party_clients client
            WHERE client.id = relying_party_client_id
              AND client.status = 'active'
        )
    );

DROP POLICY IF EXISTS relying_party_secrets_preauth_select
    ON sesame_idam.relying_party_client_secrets;
CREATE POLICY relying_party_secrets_preauth_select
    ON sesame_idam.relying_party_client_secrets
    FOR SELECT
    USING (
        COALESCE(NULLIF(current_setting('app.tenant_id', true), ''), '') = ''
        AND EXISTS (
            SELECT 1
            FROM sesame_idam.relying_party_clients client
            WHERE client.id = relying_party_client_id
              AND client.status = 'active'
              AND client.client_type = 'confidential'
        )
    );
