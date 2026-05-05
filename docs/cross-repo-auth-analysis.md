# Cross-Repo Auth Analysis & Sesame-IDAM Design Direction

> Date: 2025-06-12
> Purpose: Establish the baseline understanding of why Sesame-IDAM needs to be rebuilt, what it currently is, and where it needs to go.

---

## 1. The Problem: Three Auth Systems, Zero Alignment

There are **three separate but overlapping identity/auth systems** in the Microscaler ecosystem, each built in isolation with no coordination:

### 1a. Sesame-IDAM (this repo)
- **Scope**: Platform-wide IDAM intended for the entire Microscaler ecosystem (BRRTRouter, RERP, PriceWhisperer, Hauliage, etc.)
- **Architecture**: Two microservices — **Authentication** (identity, sessions, OIDC, JWT, token exchange) + **Authorization** (apps, roles, permissions, `principal/effective`, `authorize`)
- **Tenancy**: Organisation > Tenant hierarchy
- **API path**: `/api/v1/identity/*`, `/api/v1/am/*`, `/auth`
- **Source**: Derived from BRRTRouter canonical specs
- **Status**: Zero implementation, specs only, entity model defined in HLD

### 1b. RERP Auth (`openapi/auth/`)
- **Scope**: Auth specifically for RERP's 71 microservices
- **Architecture**: Aggregator gateway routing to **Idam** + **Rbac** sub-services
- **Tenancy**: Flat user/role/permission model, no org/tenant
- **API path**: `/api/v1/auth/idam/*`, `/api/v1/auth/rbac/*`
- **Status**: Specs exist but schemas are empty (`properties: {}`) — largely stubs

### 1c. PriceWhisperer IDAM (`openapi/trader/idam/`)
- **Scope**: Customer-facing identity for PriceWhisperer only
- **Architecture**: Wrapper/proxy around **Supabase GoTrue**
- **Tenancy**: Flat, email-centric model
- **API path**: `/api/identity/*`
- **Status**: Fully specified (3339 lines), detailed schemas with examples

### 1d. The Tension

Each repo built its own IDAM spec. Sesame has org/tenant, RERP has flat users, PriceWhisperer has email-centric `human_name`. These are not compatible. If Microscaler needs a **single auth system that any application can bolt onto** (like PropelAuth), then:

- Sesame is the right repository to build this in (platform-wide scope, org/tenant model)
- RERP's auth and PriceWhisperer's GoTrue proxy should eventually migrate to consume Sesame
- The entity model is Sesame's — it's the most complete and captures org > tenant > user > role > permission

---

## 2. The Target: A PropelAuth-like Platform

The vision is to build Sesame-IDAM as a **bolt-on identity platform** — similar to [PropelAuth](https://propelauth.com) — where:

1. An application does **not implement its own auth**
2. The application calls Sesame's platform API to manage users, orgs, roles, permissions
3. After authentication, Sesame returns a **JWT enriched with all identity/permission claims**
4. The application reads the JWT and enforces RLS / access control — zero auth logic in the app
5. Sesame supports both **customer B2B auth** (end users in orgs) and **platform admin auth** (app admins/editors)

### 2a. Core Concepts (PropelAuth-inspired)

| Concept | What It Means | Sesame Mapping |
|---------|--------------|----------------|
| **Platform Users** | People who use the application internally (admins, support) | `User` entity — differentiated by `user_type` claim in JWT |
| **Customer Users** | End users/customers of the application | `User` entity — same table, different JWT claim |
| **Organizations** | B2B customer companies with memberships | `Organization` entity with `UserOrganizationInfo` memberships |
| **Roles & Permissions** | RBAC scoped per organization | `Role`, `Permission`, `RolePermission`, `RoleInheritance` |
| **JWT Enrichment** | JWT contains user + org + role + permission claims | **MISSING** — needs to be designed |
| **Platform Admin API** | Server-side CRUD for users/orgs/roles/permissions | **PARTIALLY DEFINED** — needs completion |
| **Webhooks** | Real-time events on identity state changes | **MISSING** — needs to be designed |
| **Auth Flows** | Login, register, refresh, logout, password reset, MFA | **PARTIALLY DEFINED** — needs completion |

### 2b. What Sesame Already Has That's Valuable

The HLD entity model is genuinely good:
- Organization > Tenant hierarchy
- UserOrganizationInfo with inherited roles
- Role inheritance (parent/child)
- MFA devices, API keys, rate limiting
- Audit logs, SCIM mapping
- Session management with refresh tokens

**The entity layer is ready. The gap is entirely in the API surface and integration patterns.**

---

## 3. Critical Gaps

### Gap 1: No JWT Enrichment Endpoint
PropelAuth's killer feature: after login, the app gets a JWT with user + org + role + permissions baked in. Sesame has the data model but no endpoint that resolves and returns it.

### Gap 2: No Platform Admin REST API
PropelAuth gives full CRUD for users, orgs, memberships, roles, permissions. Sesame's auth spec frames endpoints as customer-facing, not as a platform admin API that application servers call.

### Gap 3: No Webhook System
PropelAuth sends webhooks for all identity state changes. Sesame has audit logs but no webhook delivery mechanism.

### Gap 4: Auth Flows Are Barely Defined
Login, register, refresh, logout, password reset, MFA — the specs mention OIDC/JWKS but the actual endpoints are skeletal.

### Gap 5: Fragmented API Surface
Three different path namespaces (`/api/v1/identity`, `/auth`, `/api/v1/am`) instead of a single, clear resource-based API.

### Gap 6: No SDK / Integration Pattern
PropelAuth provides SDKs with JWT validation middleware. Sesame has nothing.

---

## 4. Conclusion

Sesame-IDAM needs to be designed as a **generic identity platform service** — not just a collection of microservices. The entity model is solid. The missing pieces are:

1. Complete authentication endpoints (login, register, refresh, logout, MFA, password reset)
2. JWT enrichment/token issuance endpoint
3. Platform admin REST API (idempotent user/org/role/permission CRUD)
4. Access management endpoints (`/principal/effective`)
5. Webhook delivery system
6. Unified API surface (one namespace)
7. SDK/middleware for consuming applications

The next step is to design the complete system — data model, API contracts, JWT schema, integration patterns.

---

*See also: `docs/sesame-idam-architecture-vision.md` for the complete design.*
