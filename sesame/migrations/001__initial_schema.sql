-- Install necessary extensions
CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS pgjwt;
CREATE EXTENSION IF NOT EXISTS pgsodium;

-- ORGANIZATIONS
CREATE TABLE organizations (
                               org_id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                               org_name       TEXT        NOT NULL,
                               org_metadata   JSONB
);

-- USERS
CREATE TABLE users (
                       user_id                    UUID    PRIMARY KEY DEFAULT gen_random_uuid(),
                       legacy_user_id             TEXT,
                       email                      TEXT    NOT NULL UNIQUE,
                       email_confirmed            BOOLEAN NOT NULL DEFAULT FALSE,
                       username                   TEXT    NOT NULL UNIQUE,
                       first_name                 TEXT,
                       last_name                  TEXT,
                       picture_url                TEXT,
                       properties                 JSONB,
                       has_password               BOOLEAN NOT NULL DEFAULT FALSE,
                       update_password_required   BOOLEAN NOT NULL DEFAULT FALSE,
                       locked                     BOOLEAN NOT NULL DEFAULT FALSE,
                       enabled                    BOOLEAN NOT NULL DEFAULT TRUE,
                       mfa_enabled                BOOLEAN NOT NULL DEFAULT FALSE,
                       can_create_orgs            BOOLEAN NOT NULL DEFAULT FALSE,
                       created_at                 BIGINT  NOT NULL DEFAULT (extract(epoch FROM now())::BIGINT),
                       last_active_at             BIGINT  NOT NULL DEFAULT (extract(epoch FROM now())::BIGINT)
);

-- USER ↔ ORG INFO
CREATE TABLE user_organization_info (
                                        user_id                                UUID    NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
                                        org_id                                 UUID    NOT NULL REFERENCES organizations(org_id) ON DELETE CASCADE,
                                        user_role                              TEXT    NOT NULL,
                                        url_safe_org_name                      TEXT    NOT NULL,
                                        org_role_structure                     TEXT,
                                        inherited_user_roles_plus_current_role TEXT[]  NOT NULL DEFAULT '{}',
                                        user_permissions                       TEXT[]  NOT NULL DEFAULT '{}',
                                        additional_roles                       TEXT[]  NOT NULL DEFAULT '{}',
                                        PRIMARY KEY (user_id, org_id)
);

-- ROLES & PERMISSIONS (per-org RBAC)
CREATE TABLE roles (
                       role_id      UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                       org_id       UUID        NOT NULL REFERENCES organizations(org_id) ON DELETE CASCADE,
                       name         TEXT        NOT NULL,
                       description  TEXT,
                       created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
                       UNIQUE (org_id, name)
);

CREATE TABLE permissions (
                             permission_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                             org_id         UUID        NOT NULL REFERENCES organizations(org_id) ON DELETE CASCADE,
                             name           TEXT        NOT NULL,
                             description    TEXT,
                             created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
                             UNIQUE (org_id, name)
);

CREATE TABLE role_permissions (
                                  role_id       UUID NOT NULL REFERENCES roles(role_id) ON DELETE CASCADE,
                                  permission_id UUID NOT NULL REFERENCES permissions(permission_id) ON DELETE CASCADE,
                                  PRIMARY KEY (role_id, permission_id)
);

CREATE TABLE user_roles (
                            user_id     UUID    NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
                            role_id     UUID    NOT NULL REFERENCES roles(role_id) ON DELETE CASCADE,
                            assigned_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                            PRIMARY KEY (user_id, role_id)
);

CREATE TABLE role_inheritance (
                                  role_id        UUID NOT NULL REFERENCES roles(role_id) ON DELETE CASCADE,
                                  parent_role_id UUID NOT NULL REFERENCES roles(role_id) ON DELETE CASCADE,
                                  PRIMARY KEY (role_id, parent_role_id),
                                  CHECK (role_id <> parent_role_id)
);

-- RATE LIMITING
CREATE TABLE rate_limit_policies (
                                     policy_id   SERIAL      PRIMARY KEY,
                                     name        TEXT        NOT NULL,
                                     scope       TEXT        NOT NULL CHECK (scope IN ('user','org','ip','global')),
                                     limit_count INTEGER     NOT NULL,
                                     window_sec  INTEGER     NOT NULL,
                                     description TEXT,
                                     created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE rate_limit_assignments (
                                        policy_id INT  NOT NULL REFERENCES rate_limit_policies(policy_id) ON DELETE CASCADE,
                                        user_id   UUID REFERENCES users(user_id) ON DELETE CASCADE,
                                        org_id    UUID REFERENCES organizations(org_id) ON DELETE CASCADE,
                                        PRIMARY KEY (policy_id, user_id, org_id)
);

CREATE TABLE rate_limit_counters (
                                     policy_id    INT        NOT NULL REFERENCES rate_limit_policies(policy_id) ON DELETE CASCADE,
                                     user_id      UUID       REFERENCES users(user_id) ON DELETE CASCADE,
                                     org_id       UUID       REFERENCES organizations(org_id) ON DELETE CASCADE,
                                     window_start TIMESTAMPTZ NOT NULL,
                                     count        INTEGER    NOT NULL DEFAULT 0,
                                     last_updated TIMESTAMPTZ NOT NULL DEFAULT now(),
                                     PRIMARY KEY (policy_id, user_id, org_id, window_start)
);

-- SESSIONS & TOKENS
CREATE TABLE sessions (
                          session_id    UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                          user_id       UUID        NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
                          refresh_token TEXT        NOT NULL,
                          created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
                          expires_at    TIMESTAMPTZ NOT NULL,
                          revoked       BOOLEAN     NOT NULL DEFAULT FALSE,
                          ip_address    INET,
                          user_agent    TEXT
);

CREATE TABLE password_reset_tokens (
                                       token      UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                                       user_id    UUID        NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
                                       created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                                       expires_at TIMESTAMPTZ NOT NULL,
                                       used_at    TIMESTAMPTZ
);

CREATE TABLE email_verification_tokens (
                                           token      UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                                           user_id    UUID        NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
                                           created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                                           expires_at TIMESTAMPTZ NOT NULL,
                                           used_at    TIMESTAMPTZ
);

-- SECURITY & RATE-INSIGHTS
CREATE TABLE login_attempts (
                                attempt_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                                user_id    UUID        REFERENCES users(user_id),
                                org_id     UUID        REFERENCES organizations(org_id),
                                timestamp  TIMESTAMPTZ NOT NULL DEFAULT now(),
                                success    BOOLEAN     NOT NULL,
                                ip_address INET,
                                user_agent TEXT
);

CREATE TABLE waitlist_signups (
                                  signup_id  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                                  email      TEXT        NOT NULL,
                                  invited    BOOLEAN     NOT NULL DEFAULT FALSE,
                                  invited_at TIMESTAMPTZ
);

-- API KEYS
CREATE TABLE api_keys (
                          api_key_id  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                          user_id     UUID        NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
                          key_hash    TEXT        NOT NULL,
                          description TEXT,
                          created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
                          expires_at  TIMESTAMPTZ,
                          revoked     BOOLEAN     NOT NULL DEFAULT FALSE
);

-- LINK API KEYS TO RATE-LIMIT POLICIES
CREATE TABLE api_key_rate_limits (
                                     api_key_id UUID NOT NULL REFERENCES api_keys(api_key_id) ON DELETE CASCADE,
                                     policy_id  INT  NOT NULL REFERENCES rate_limit_policies(policy_id) ON DELETE CASCADE,
                                     PRIMARY KEY (api_key_id, policy_id)
);

-- MFA DEVICES
CREATE TABLE mfa_devices (
                             device_id   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                             user_id     UUID        NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
                             type        TEXT        NOT NULL,
                             secret      TEXT,
                             created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
                             enabled     BOOLEAN     NOT NULL DEFAULT TRUE,
                             last_used   TIMESTAMPTZ
);

-- SSO / IDENTITY PROVIDERS
CREATE TABLE identity_providers (
                                    provider_id UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                                    org_id       UUID        NOT NULL REFERENCES organizations(org_id) ON DELETE CASCADE,
                                    type         TEXT        NOT NULL,
                                    name         TEXT        NOT NULL,
                                    config       JSONB       NOT NULL,
                                    enabled      BOOLEAN     NOT NULL DEFAULT FALSE,
                                    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- SCIM CLIENTS & MAPPINGS
CREATE TABLE scim_clients (
                              client_id     UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                              org_id        UUID        NOT NULL REFERENCES organizations(org_id) ON DELETE CASCADE,
                              client_secret TEXT        NOT NULL,
                              created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
                              expires_at    TIMESTAMPTZ,
                              enabled       BOOLEAN     NOT NULL DEFAULT TRUE
);

CREATE TABLE scim_mappings (
                               mapping_id     UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                               client_id      UUID        NOT NULL REFERENCES scim_clients(client_id) ON DELETE CASCADE,
                               local_attribute TEXT       NOT NULL,
                               scim_attribute  TEXT       NOT NULL
);

CREATE TABLE scim_tokens (
                             token_id   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                             client_id  UUID        NOT NULL REFERENCES scim_clients(client_id) ON DELETE CASCADE,
                             token      TEXT        NOT NULL,
                             expires_at TIMESTAMPTZ NOT NULL
);

-- ORG-SCOPED LOGIN METHODS
CREATE TABLE org_login_methods (
                                   org_id  UUID    NOT NULL REFERENCES organizations(org_id) ON DELETE CASCADE,
                                   method  TEXT    NOT NULL,
                                   enabled BOOLEAN NOT NULL DEFAULT TRUE,
                                   PRIMARY KEY (org_id, method)
);

-- AUDIT & IMPERSONATION
CREATE TABLE audit_logs (
                            log_id      UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                            user_id     UUID        REFERENCES users(user_id),
                            org_id      UUID        REFERENCES organizations(org_id),
                            action      TEXT        NOT NULL,
                            object_type TEXT,
                            object_id   TEXT,
                            ip_address  INET,
                            user_agent  TEXT,
                            timestamp   TIMESTAMPTZ NOT NULL DEFAULT now(),
                            details     JSONB
);

-- IMPERSONATION LOGS
CREATE TABLE impersonation_logs (
                                    log_id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                                    admin_user_id        UUID        NOT NULL REFERENCES users(user_id),
                                    impersonated_user_id UUID        NOT NULL REFERENCES users(user_id),
                                    started_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
                                    ended_at             TIMESTAMPTZ,
                                    ip_address           INET,
                                    user_agent           TEXT
);

-- METRICS / USER INSIGHTS & EVENTS
CREATE TABLE metrics_events (
                                event_id       UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                                user_id        UUID        REFERENCES users(user_id),
                                org_id         UUID        REFERENCES organizations(org_id),
                                event_type     TEXT        NOT NULL,
                                event_timestamp TIMESTAMPTZ NOT NULL DEFAULT now(),
                                metadata       JSONB
);
