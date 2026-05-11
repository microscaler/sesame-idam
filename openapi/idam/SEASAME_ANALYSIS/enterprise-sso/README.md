# Enterprise SSO

> **Component:** SAML 2.0 and OIDC identity provider federation, identity brokering, just-in-time provisioning
> **Priority:** P2 — Required for enterprise B2B sales and corporate identity integration
> **Service:** org-mgmt (SSO configuration endpoints, 43 endpoints total)

---

## The Pitch

**Buyer Question:** *Can I connect to existing identity providers — Okta, Azure AD, Google Workspace, PingIdentity — and let employees sign in with their corporate credentials using SAML 2.0 or OIDC?*

If the answer is no, you're not an enterprise product — you're a tool that requires separate user accounts for every organization that adopts you. Enterprise SSO (Single Sign-On) is the bridge between your application and the corporate identity infrastructure that employees already use. Without it, every new customer onboarding requires manual user creation, password sharing, and IT friction. With it, adoption is automatic: the customer configures their IdP once, and all employees can sign in immediately.

---

## What This Component Does

Enterprise SSO enables identity federation between Sesame-IDAM and external identity providers:

1. **SAML 2.0 Identity Provider Integration** — Configure and validate SAML assertions from enterprise IdPs (Okta, Azure AD, PingFederate, OneLogin)
2. **OIDC Identity Provider Integration** — Support OpenID Connect providers for modern federated login
3. **Just-in-Time (JIT) Provisioning** — Automatically create user accounts in Sesame-IDAM upon first login via enterprise IdP
4. **Attribute Mapping** — Map IdP attributes (email, name, groups) to Sesame-IDAM user profile fields
5. **Assertion Consumer Service (ACS)** — Handle SAML assertion responses with signature validation and encryption
6. **Single Logout (SLO)** — Coordinate logout across the application and IdP
7. **Metadata Discovery** — Fetch and validate IdP metadata from URLs (SAML) or well-known endpoints (OIDC)
8. **Identity Brokering** — Route authentication requests to the correct IdP based on domain or user request
9. **Directory Sync via SCIM** — Synchronize users and groups from enterprise directories (implemented in org-mgmt)
10. **Domain-Based Routing** — Auto-discover the correct IdP based on user's email domain

---

## Entity Model

### SSO Configuration Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | SSO configuration identifier |
| `org_id` | UUID | Yes | Associated organization |
| `provider` | Enum: [saml, oidc] | Yes | Identity provider type |
| `entity_id` | String (255) | No | SAML EntityID / OIDC Issuer |
| `sso_url` | String (1024) | Yes | IdP SSO endpoint URL |
| `slo_url` | String (1024) | No | IdP SLO endpoint URL |
| `metadata_url` | String (1024) | No | IdP metadata document URL |
| `signing_certificate` | String (2048) | No | IdP X.509 signing certificate |
| `encryption_certificate` | String (2048) | No | IdP encryption certificate |
| `sp_entity_id` | String (255) | Yes | Sesame SP EntityID |
| `acs_url` | String (512) | Yes | Sesame ACS endpoint |
| `slo_redirect_url` | String (512) | No | Sesame SLO redirect URL |
| `name_id_format` | Enum: [email, transient, persistent] | Yes | NameID format for SAML |
| `attribute_mapping` | JSON | Yes | IdP attribute → Sesame field mapping |
| `jti_claims` | Array[String] | Yes | JWT claims to extract |
| `signing_key` | String (4096) | No | SP private key for signing |
| `signature_algorithm` | Enum: [RSA_SHA256, RSA_SHA384, RSA_SHA512] | Yes | SAML signature algorithm |
| `is_active` | Boolean | Yes | Whether SSO is enabled |
| `is_default` | Boolean | No | Default IdP for the org |
| `domain_patterns` | Array[String] | No | Email domain patterns for routing |

### JIT Provisioning Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | JIT provisioning configuration |
| `org_id` | UUID | Yes | Associated organization |
| `enabled` | Boolean | Yes | Whether JIT is enabled |
| `auto_create_groups` | Boolean | No | Auto-create groups from IdP |
| `default_role_id` | UUID | No | Default role for new users |
| `group_mapping` | JSON | No | IdP group → Sesame role mapping |
| `created_at` | DateTime | Yes | Creation timestamp |

### Identity Broker Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | Broker configuration |
| `org_id` | UUID | Yes | Associated organization |
| `domain` | String (255) | Yes | Email domain pattern |
| `idp_id` | UUID | Yes | Associated IdP configuration |
| `is_active` | Boolean | Yes | Whether domain routing is active |
| `created_at` | DateTime | Yes | Creation timestamp |

---

## Entity Relationships

```
SSOConfiguration ───┬── Organization (via org_id)       ← SSO owner
                    ├── IdentityBroker (one2many)       ← Domain routing
                    ├── JITProvisioning (one2one)       ← Auto-provisioning
                    └── User (many2many via login)     ← Users who logged in via SSO
```

---

## Required API Endpoints

### SSO Configuration

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/orgs/{id}/sso/configure` | Configure SAML/OIDC identity provider |
| `POST` | `/api/v1/orgs/{id}/sso/test` | Test SSO configuration |
| `GET` | `/api/v1/orgs/{id}/sso/config` | Get SSO configuration |
| `POST` | `/api/v1/orgs/{id}/sso/config` | Update SSO configuration |
| `DELETE` | `/api/v1/orgs/{id}/sso/config` | Disable/delete SSO configuration |

### SSO Authentication Flow

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/sso/authorize` | Initiate SSO authorization request |
| `GET` | `/api/v1/sso/saml/acs` | SAML Assertion Consumer Service endpoint |
| `POST` | `/api/v1/sso/saml/acs` | SAML POST binding ACS |
| `GET` | `/api/v1/slo` | SAML Single Logout endpoint |
| `GET` | `/api/v1/sso/oidc/callback` | OIDC authorization callback |

### Identity Broker

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/sso/broker/{domain}` | Discover IdP for email domain |
| `GET` | `/api/v1/sso/metadata` | Get SP metadata for IdP configuration |
| `GET` | `/api/v1/sso/.well-known/openid-configuration` | OIDC discovery endpoint |

### JIT Provisioning

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/orgs/{id}/jit/config` | Get JIT provisioning config |
| `POST` | `/api/v1/orgs/{id}/jit/config` | Update JIT provisioning config |

### SCIM Directory Sync

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/orgs/{id}/scim/sync` | Trigger directory sync |
| `GET` | `/api/v1/orgs/{id}/scim/sync/status` | Get sync status |
| `POST` | `/api/v1/orgs/{id}/scim/sync/configure` | Configure SCIM provider |

---

## Competitive Positioning

### Where Sesame-IDAM Wins
- **API-driven SSO configuration** — Every SSO setting is configurable via REST. No admin console UI dependency.
- **Tenant-scoped SSO** — Each tenant can have different IdP configurations.
- **Built-in SCIM** — Directory synchronization is built in, not a separate product.
- **Rust-native assertion validation** — SAML assertion parsing and validation in Rust is faster and more memory-safe.

### Where Sesame-IDAM Lags
- **No SSO dashboard** — No visual SSO configuration UI. Auth0 and Okta provide guided SSO setup.
- **No SAML metadata generation** — No downloadable IdP metadata for configuring enterprise IdPs.
- **No SLO (Single Logout)** — Logout coordination across IdP and application is not implemented.
- **No IdP discovery** — No automatic IdP discovery based on email domain.
- **No SCIM 2.0 endpoint** — SCIM sync is trigger-based, not reactive to IdP changes.

---

## Competitive Intelligence Deep Dive

### Auth0: Universal Identity Broker
Auth0's Enterprise SSO supports 100+ SAML providers with pre-built templates. The Enterprise Connection feature handles JIT provisioning, attribute mapping, and domain-based routing out of the box. **Sesame Gap:** No pre-built templates, no domain routing, no guided setup.

### Okta: SAML with Just-in-Time Provisioning
Okta's SAML integration supports JIT provisioning, attribute mapping, and group-to-role mapping. Okta also supports SAML 2.0 PDP (Policy Decision Point) for fine-grained authorization. **Sesame Gap:** Okta's SAML setup is visual and guided. Sesame requires manual configuration.

### Microsoft Entra ID: SAML SSO at Scale
Entra ID supports SAML SSO for 4,000+ SaaS applications with automatic attribute provisioning and conditional access policies. **Sesame Gap:** Entra's ecosystem is unmatched — Sesame is one of 4,000 apps, not a provider itself.

### PingIdentity: Enterprise SAML
PingFederate is the enterprise SAML standard, supporting complex attribute transformation, identity federation, and policy enforcement. **Sesame Gap:** Ping is the gold standard for SAML. Sesame's SAML support is minimal.

---

## Implementation Roadmap

### Phase 1: Basic SAML (Not Implemented) — P2
1. SAML 2.0 service provider configuration
2. ACS endpoint with signature validation
3. Basic attribute mapping (email, name)
4. SAML metadata generation for IdP configuration

### Phase 2: OIDC & JIT (Not Implemented) — P2
1. OIDC provider integration
2. Just-in-time user provisioning
3. Group-to-role mapping from IdP claims
4. Domain-based IdP discovery and routing

### Phase 3: Enterprise SSO (Not Implemented) — P3
1. SAML Single Logout (SLO)
2. Advanced attribute transformation
3. SAML 2.0 Identity Provider mode (Sesame as IdP for customer apps)
4. SCIM 2.0 reactive provisioning
5. IdP metadata templates for major providers (Okta, Azure AD, Ping)

---

## Key Takeaway for Buyers

Enterprise SSO is the **most critical gap for B2B sales**. Without SAML/OIDC integration, Sesame-IDAM cannot be deployed as a corporate application behind existing identity infrastructure. Every enterprise buyer requires SSO as a non-negotiable requirement.

**For consumer-facing applications**, SSO is not critical — email/password and social OAuth are sufficient. **For B2B and enterprise applications**, SSO is the gatekeeper. Without it, the product cannot be adopted by organizations with existing identity providers.

**Immediate priority:** Implement SAML 2.0 service provider mode within 8 weeks. This alone opens Sesame-IDAM to the entire enterprise market.
