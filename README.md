![Sesame Logo](./ui/images/logo.png)

# Sesame-IDAM

> **Open-source bolt-on identity & access management for B2B SaaS.**  
> Zero auth logic in your app. Deployed as four independent Rust microservices.  
> Matches the full PropelAuth API surface with native PostgreSQL RLS security.

[Design Document](docs/design-doc.md) · [Gap Analysis](docs/propelauth-gap-analysis.md) · [Service Topology](docs/service-topology-design.md) · [RLS Integration](docs/rls-design-v2.md)

---

## What Is Sesame-IDAM?

Sesame-IDAM is an **open-source bolt-on identity platform** that any B2B SaaS application can integrate in hours — not months. The consuming application implements **zero authentication logic**.

Sesame manages:
- **Users** — create, fetch, update, delete, search, invite, impersonate
- **Organizations** — org lifecycle, memberships, invites, seat limits, domain controls
- **Roles & Permissions** — RBAC with inheritance, ABAC attributes, fine-grained checks
- **Sessions** — login, refresh, logout, token rotation, MFA, social login
- **API Keys** — M2M authentication for services and CLI tools
- **Enterprise SSO** — SAML/OIDC per organization
- **Webhooks** — real-time event delivery for identity state changes
- **MCP** — Model Context Protocol authentication for AI agents

The application receives **enriched JWTs** containing all identity, org, role, and permission claims. It reads the JWT and enforces access control — route-level, RLS, feature flags. **Nothing else is needed.**

---

## Why Sesame?

| Feature | **PropelAuth** | **Supabase Auth** | **Sesame-IDAM** |
|---------|---------------|-------------------|----------------|
| Core promise | "Auth for your SaaS" | "Auth for your database" | **"Auth for your SaaS with database-level security"** |
| User/org model | Built-in B2B | Flat users | **Built-in B2B (same as PropelAuth)** |
| Database security | JWT claims only | Native RLS | **Native RLS helpers (we provide the SQL)** |
| Integration | Backend API + Frontend SDK | SDK + PostgREST | **Backend API + SDK + RLS Helper SQL** |
| Source | Proprietary / Paid | Open source | **Open source (Rust), self-hosted** |
| Per-user pricing | Yes | Yes | **No — free, self-hosted** |
| Vendor lock-in | Yes | Yes | **None** |

Sesame combines the **B2B complexity of PropelAuth** with the **database-native security of Supabase** — delivered as open-source infrastructure.

---

## Architecture

Sesame is **four independent Rust microservices**, not a monolith. Split by per-request frequency and cost:

```mermaid
graph TB
    subgraph "Consumer Applications"
        SPA[Single-Page Apps / Mobile]
        Microsvc[Microservices / M2M]
        Admin[Admin Dashboards]
    end

    subgraph "API Gateway / Proxy"
        Proxy[Route by path prefix]
    end

    subgraph "Sesame-IDAM Services"
        IA[identity-auth<br/>Port 8001<br/>HIGH frequency]
        AC[authz-core<br/>Port 8002<br/>EXTREME frequency]
        AK[api-keys<br/>Port 8003<br/>HIGH frequency]
        OM[org-mgmt<br/>Port 8004<br/>LOW frequency]
    end

    subgraph "Storage"
        PG[(PostgreSQL)]
        Redis[(Redis)]
    end

    SPA --> Proxy
    Microsvc --> Proxy
    Admin --> Proxy
    Proxy --> IA
    Proxy --> AC
    Proxy --> AK
    Proxy --> OM

    IA -. login calls .-> AC
    AC -. cache .-> Redis
    IA -. session cache .-> Redis
    AC -. role/perm cache .-> Redis
    AK -. validation cache .-> Redis
    IA -. user/session data .-> PG
    AC -. role/perm definitions .-> PG
    AK -. key data .-> PG
    OM -. org/role/perm data .-> PG
```

### Service Breakdown

| Service | Base Path | Frequency | Purpose |
|---------|-----------|-----------|---------|
| **identity-auth** | `/auth/*`, `/api/v1/identity/*`, `/.well-known/*` | HIGH | Login, register, refresh, logout, MFA, user CRUD, OIDC, JWKS, MCP |
| **authz-core** | `/api/v1/am/authorize`, `/api/v1/am/principal/*` | EXTREME | Per-request authorization, principal/effective, role evaluation |
| **api-keys** | `/api/v1/am/api-keys/*` | HIGH | API key lifecycle, validation (personal + org), rotation, archival |
| **org-mgmt** | `/orgs/*`, `/api/v1/am/applications/*` | LOW | Org lifecycle, memberships, SSO/SCIM, roles, permissions, webhooks |

---

## The RLS Bridge — Our Killer Feature

PropelAuth gives you the JWT. Supabase gives you RLS helpers. **Sesame gives you both.**

```mermaid
graph TB
    subgraph "Layer 1: Application Server"
        App["App Server<br/>Receives request with Bearer token<br/>Validates JWT via SesameAuthMiddleware<br/>Extracts claims: user_id, org_id, user_type, perms"]
    end

    subgraph "Layer 2: SesameExecutor (Lifeguard ORM)"
        SE["SesameExecutor wraps LifeExecutor<br/>Automatically runs sesame_set_session()<br/>SET LOCAL auth.user_org_id = 'uuid'<br/>Session-scoped — cleared on transaction end"]
    end

    subgraph "Layer 3: PostgreSQL RLS"
        PG[(PostgreSQL RLS Policies)]
        RLS["USING (org_id = sesame_current_user_org_id())<br/>USING (sesame_current_user_type() = 'customer')<br/>Failsafe: NULLIF returns NULL → zero rows"]
    end

    App --> SE --> PG --> RLS
```

The application validates the JWT in the application layer. SesameExecutor automatically calls `SET LOCAL` at the start of every database transaction. RLS policies reference `sesame_current_user_org_id()` to filter rows. The JWT itself **never enters the database**.

### SQL Helpers (Deployed Once Into Your DB)

```sql
CREATE OR REPLACE FUNCTION public.sesame_set_session(
    p_user_id uuid, p_user_org_id uuid,
    p_user_org_type text DEFAULT 'consumer',
    p_user_type text DEFAULT 'customer',
    p_permissions text[] DEFAULT '{}',
    p_user_email text DEFAULT NULL
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    SET LOCAL auth.user_id := p_user_id;
    SET LOCAL auth.user_org_id := p_user_org_id;
    SET LOCAL auth.user_org_type := p_user_org_type;
    SET LOCAL auth.user_type := p_user_type;
    SET LOCAL auth.permissions := p_permissions;
    SET LOCAL auth.user_email := p_user_email;
END;
$$;
```

Deploy this once per consuming application's database. All subsequent queries are automatically org-scoped.

---

## Developer Contract

When a developer integrates Sesame, this is exactly what they interact with:

### Frontend (User-Facing)
```typescript
import { useAuth } from '@sesame-idam/frontend';

function App() {
  const { user, orgs, isLoading, login, logout } = useAuth();
  // Never write login logic — Sesame handles it
}
```

### Backend Admin API (Server-Facing)
```typescript
// Manage users, orgs, memberships, roles, permissions
const users = await sesame.users.list({ limit: 10 });
const org = await sesame.orgs.get('org_xyz789');
await sesame.orgs.update('org_xyz789', {
  name: 'Acme Corp 2.0',
  settings: { max_users: 100, password_rotation_enabled: true }
});
const members = await sesame.orgs.getMembers('org_xyz789');
await sesame.orgs.addMember('org_xyz789', { userId: 'user_abc', role: 'Admin' });
```

### Database (Automatic RLS)
```typescript
// All queries are automatically org-scoped via SesameExecutor
// No changes needed to application code
const rows = await db.query('SELECT * FROM my_custom_table');
// RLS policy fires automatically: USING (org_id = sesame_current_user_org_id())
```

---

## JWT Schema

Every token issued by Sesame is an RS256-signed JWT containing everything the application needs:

```json
{
  "sub": "31c41c16-...",
  "user_id": "31c41c16-...",
  "user_type": "customer",
  "org_id": "1189c444-...",
  "org_name": "Acme Inc",
  "roles": ["admin", "billing-viewer"],
  "permissions": ["org:admin", "billing:read", "billing:write"],
  "mfa_enabled": true,
  "is_platform_admin": false,
  "email_verified": true,
  "locked": false,
  "enabled": true
}
```

All claims are **authoritative in Sesame** — written at token generation, never modifiable by the client.

**Coarse-grained checks** (e.g., "is Admin?") use JWT claims directly — zero latency. **Fine-grained checks** (e.g., "can user delete invoice #123?") call `POST /authorize` with ABAC rules.

---

## OpenAPI Surface

| Service | Specs | Endpoints | Schemas |
|---------|-------|-----------|---------|
| identity-auth | `openapi/identity-auth/openapi.yaml` + 3 sub-specs | 48 | 43 |
| authz-core | `openapi/authz-core/openapi.yaml` | 5 | 8 |
| api-keys | `openapi/api-keys/openapi.yaml` | 10 | 15 |
| org-mgmt | `openapi/org-mgmt/openapi.yaml` | 38 | 37 |
| **Total** | **7 files** | **146 endpoints** | **152 schemas** |

The canonical combined spec (`identity-auth/openapi.yaml`) feeds BRRTRouter codegen. Sub-specs are self-contained copies for navigation.

---

## Sesame-Only Features

| Feature | Description |
|---------|-------------|
| **RLS Helper SQL** | `sesame_set_session()`, `sesame_current_*()` — database-level security |
| **SesameExecutor** | Automatic RLS injection at ORM level via Lifeguard wrapper |
| **Dual OTP** | Email + phone simultaneous verification |
| **Phone OTP** | SMS OTP login |
| **Role inheritance** | Explicit `parent_role_id` in data model |
| **Application model** | First-class Application entities |
| **Webhook system** | Complete delivery with retries, HMAC signing, tracking |
| **User type** | `customer` / `platform` distinction at JWT claim level |
| **Token rotation** | Explicit refresh token rotation on every `/refresh` |
| **org_type** | Provider/consumer/platform persona classification |
| **MCP support** | Model Context Protocol authentication for AI agents |
| **Open source** | Self-hosted, no per-user pricing, no vendor lock-in |

---

## Getting Started

### Prerequisites
- Rust toolchain
- Docker + Docker Compose (or Kind cluster)
- PostgreSQL (or Tilt-deployed Supabase stack)
- Redis

### Quick Setup
```bash
just init        # Create tooling venv
just dev-up      # Start Kind cluster + Tilt (port 10351)
just supabase-apply  # Deploy Supabase stack (PostgreSQL)
just port-forward    # Forward postgres + redis to localhost
```

### Build & Generate
```bash
just gen         # Generate crates from OpenAPI specs
just gen-auth    # Generate identity-auth crate only
just lint-openapi  # Lint all specs via brrtrouter-gen
just serve-auth    # Start echo server for local testing
```

### Guard Rails
- **Ruff** — same select/ignore and complexity limits as RERP
- **Pre-commit** — `just qa` (lint + format-check + pytest) before every commit
- Install hooks: `just install-hooks`

---

## Status

- **OpenAPI:** 146 endpoints across 7 spec files — **100% coverage** of PropelAuth API surface plus 11 Sesame-only features
- **Gap Analysis:** [docs/propelauth-gap-analysis.md](docs/propalauth-gap-analysis.md) — full line-by-line comparison
- **Design:** [docs/design-doc.md](docs/design-doc.md) — comprehensive architecture, data model, security, integration patterns
- **Rust:** Zero implementation currently. `gen/` + `impl/` crates per microservice planned (BRRTRouter codegen + Lifeguard ORM)
- **Kubernetes:** Helm charts and Tiltfile ready for deployment once implementations exist
- **Archive:** Pre-pivot state preserved on branch `archive/saas-idam-pre-microservice-pivot`

---

## Links

- [Full Design Document](docs/design-doc.md)
- [Gap Analysis vs PropelAuth](docs/propalauth-gap-analysis.md)
- [Service Topology](docs/service-topology-design.md)
- [RLS Integration Design](docs/rls-design-v2.md)
- [OpenAPI Specs](openapi/README.md)
- [AGENTS.md](AGENTS.md) — developer notes and tooling

---

*Sesame-IDAM is part of the Microscaler ecosystem — alongside BRRTRouter (API gateway), RERP (resource planning), PriceWhisperer (market data), and Hauliage (logistics).*
