-- Trusted relying-party directory bridge (PRD-OPENGROUPWARE F3).
--
-- Read-only views over sesame identity data for FIRST-PARTY protocol
-- servers that verify user credentials natively against SQL (first
-- consumer: Stalwart's SQL directory for IMAP/SMTP/DAV auth on the
-- opengroupware mail platform).
--
-- Security model:
--  * Each relying party gets its OWN LOGIN role, granted rp_directory_read;
--    SELECT-only on these views, nothing else. Created per environment,
--    e.g.: CREATE ROLE rp_stalwart LOGIN PASSWORD '...';
--          GRANT rp_directory_read TO rp_stalwart;
--  * Views are SECURITY DEFINER-equivalent via a dedicated owner role that
--    carries BYPASSRLS: relying parties authenticate users across the whole
--    tenant population by design (an IMAP login arrives before any tenant
--    context exists). The exposure is bounded by the view definitions:
--    active tenants, active users, PHC hashes only (argon2id — offline-
--    attack resistant), no PII beyond login identity.
--  * Revocation = revoke the RP role. Auditing = Postgres log_connections
--    on the RP roles + pgaudit if enabled.

CREATE SCHEMA IF NOT EXISTS rp_directory;

-- Owner role for the views: no login, BYPASSRLS so the views see all
-- tenants' rows (views run with the owner's RLS context).
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'rp_directory_owner') THEN
        CREATE ROLE rp_directory_owner NOLOGIN BYPASSRLS;
    END IF;
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'rp_directory_read') THEN
        CREATE ROLE rp_directory_read NOLOGIN;
    END IF;
END
$$;

GRANT USAGE ON SCHEMA sesame_idam TO rp_directory_owner;
GRANT SELECT ON sesame_idam.users, sesame_idam.tenants,
               sesame_idam.app_passwords TO rp_directory_owner;

-- Primary credentials: one row per active user in an active tenant.
-- Stalwart SQL directory query shape:
--   SELECT secret_phc FROM rp_directory.users WHERE login = $1
-- users.tenant_id is the tenant slug (the registry key in
-- sesame_idam.tenants.slug).
CREATE OR REPLACE VIEW rp_directory.users AS
SELECT
    t.slug                 AS tenant_slug,
    u.tenant_id            AS tenant_id,
    u.email                AS login,
    u.password_hash        AS secret_phc,
    u.email                AS display_name,
    u.id                   AS user_id
FROM sesame_idam.users u
JOIN sesame_idam.tenants t ON t.slug = u.tenant_id
WHERE u.status = 'active'
  AND t.status = 'active';

ALTER VIEW rp_directory.users OWNER TO rp_directory_owner;

-- App passwords: multiple rows per user; RPs verify against ANY active row
-- (fallback order: app passwords first, then primary — RP's choice).
CREATE OR REPLACE VIEW rp_directory.app_passwords AS
SELECT
    t.slug          AS tenant_slug,
    u.tenant_id     AS tenant_id,
    u.email         AS login,
    ap.secret_phc   AS secret_phc,
    ap.scopes       AS scopes,
    ap.id           AS app_password_id
FROM sesame_idam.app_passwords ap
JOIN sesame_idam.users u   ON u.id = ap.user_id
JOIN sesame_idam.tenants t ON t.slug = u.tenant_id
WHERE ap.revoked_at IS NULL
  AND u.status = 'active'
  AND t.status = 'active';

ALTER VIEW rp_directory.app_passwords OWNER TO rp_directory_owner;

GRANT USAGE ON SCHEMA rp_directory TO rp_directory_read;
GRANT SELECT ON rp_directory.users, rp_directory.app_passwords
    TO rp_directory_read;
