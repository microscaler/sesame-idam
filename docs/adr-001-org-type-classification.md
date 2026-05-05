# ADR-001: Org Type Classification in B2B SaaS

> **Status:** Proposed
> **Date:** 2026-01-04
> **Deciders:** Engineering, Product
> **Re-supposed By:** ADR-001

---

## 1. Context

Sesame-IDAM supports B2B SaaS applications where **three distinct organisation personas** coexist on the same platform:

1. **Platform Org** — The SaaS operator itself (the company selling the platform)
2. **Service Provider Org** — Business entities that deliver services through the platform (e.g. employment agencies, transporters, brokers)
3. **Service Consumer Org** — Business entities that consume services (e.g. employing companies, shippers, buyers)

These personas have fundamentally different capabilities, access patterns, and data boundaries. A Platform org admin can manage provider organisations. A Provider org admin can invite consumer orgs and manage relationships between them. A Consumer org admin can only manage their own org's membership and consumption of provider services.

Currently, the Org schema in Sesame-IDAM has **no mechanism to differentiate between these personas**. An org is just an org. The only distinction the API enforces is Platform Admin vs SaaS Customer, which is a *user-level* check, not an *org-level* check.

## 2. Constraints

- **No URI differentiation** — `org_type` must never appear in the URI path (e.g. `/provider-orgs/` or `/consumer-orgs/`). This would leak a trusted attribute into a client-controlled context, inviting tampering and privilege escalation.
- **No org hierarchy** — Orgs are flat siblings. A Provider org does not "own" Consumer orgs in a parent/child relationship.
- **Membership model remains org-to-user** — Users are members of orgs. There is no org-to-org relationship primitive at the IDAM level. Cross-org interactions (provider offering services to consumers) are application-layer concerns.
- **The differentiation must be authoritative** — The platform must be able to enforce org-type rules in its own service logic and propagate trust (e.g. via JWT claims) to downstream services.

## 3. Options

### Option A: Enforce Org Type in Platform Service Only

Add an `org_type` field to the Org entity and API, but enforce it exclusively within Sesame-IDAM's platform service. Downstream microservices (authz-core, org-mgmt) receive the org_type as a trusted claim in the JWT but never query or modify it directly. The platform service is the sole authority on org creation and type assignment.

**Flow:**

```
Platform Admin (org_type: platform)
    └── POST /orgs (via platform service, org_type: provider OR consumer)
         └── JWT issued to users of that org contains `org_type` claim
              └── authz-core reads org_type from JWT, enforces rules
```

**Pros:**
- Clean separation: platform service owns org lifecycle, other services are org-agnostic consumers of JWT claims
- No circular dependency: org-mgmt and authz-core don't need to know about org_type enforcement
- Matches the existing 4-service topology where platform service handles platform-specific operations
- JWT is the canonical trust boundary — org_type as a claim is already the pattern used for `user_type` (platform_user vs customer)

**Cons:**
- Requires downstream services to validate JWT claims rather than calling org-mgmt API to check org_type
- If a downstream service needs to list/filter by org_type, it must do it at the application layer using JWT context, not API filters

### Option B: Expose Org Type as a First-Class API Primitive

Add `org_type` as a field on the Org schema in the canonical spec, with CRUD operations and query filters available through the org-mgmt service. Any service can query and modify org types.

**Pros:**
- All services can independently check and filter by org_type
- Transparent and self-documenting in the OpenAPI spec

**Cons:**
- Violates the constraint of not putting trusted data in client-controlled context — org_type becomes queryable via API
- Risk of privilege escalation: a Consumer org could call `GET /orgs?org_type=provider` or attempt `PATCH /orgs/{id}` to reclassify themselves
- Breaks the principle that only the platform controls org lifecycle
- Creates circular dependency: org-mgmt would need to check the caller's org_type to enforce who can modify what

### Option C: Hybrid — Read-Only Claim, Write via Platform Service Only

org_type exists as a JWT claim readable by any service, but write access (creation and modification) is gated behind a dedicated platform-only API endpoint that rejects all non-platform requests. The org-mgmt service never exposes org_type in response schemas to non-platform clients.

**Pros:**
- Defence in depth: even if a service misbehaves, it can't expose org_type to consumers
- Explicit trust boundary

**Cons:**
- Complicates the API contract — the same org object looks different depending on the caller's token type
- Violates API consistency principles
- Harder to debug and test

## 4. Decision

**We choose Option A.**

Add `org_type` as a field on the Org entity and schema. The platform service owns all write operations on this field (creation and updates). The org-mgmt and authz-core services read org_type exclusively from the JWT claim — they never expose or modify it directly via API.

This aligns with the existing design principle that **JWT claims are the canonical trust boundary** (already used for `user_type`, `roles`, `permissions`). org_type is the org-level analogue of user_type.

## 5. Implications

### 5.1 Org Schema Change

The `Org` schema gains an `org_type` field:

```yaml
Org:
  type: object
  properties:
    # ... existing fields ...
    org_type:
      type: string
      enum: [platform, provider, consumer]
      description: |
        Organisation type classification. Set by platform service at creation.
        Never modifiable by non-platform users. Read via JWT claim.
      readOnly: true
```

### 5.2 Platform Service Responsibilities

- **create_org** — accepts a required `org_type` parameter. Platform admin can create any type. Only platform-level users can call this endpoint.
- **update_org_type** — a dedicated platform-only endpoint for reclassification. No generic PATCH to orgs for org_type modification.
- All responses to non-platform clients still include org_type (it's metadata, not a secret), but write operations are rejected.

### 5.3 JWT Claim

The JWT issued to every user includes:

```json
{
  "user_type": "customer",
  "org_id": "abc-123",
  "org_type": "provider",
  "roles": ["member", "admin"],
  "permissions": [...]
}
```

org_type is a **trusted claim** — set by the identity service at token generation, never modifiable by the client.

### 5.4 Authz-Core Service Changes

- The `authorize` operation reads org_type from the JWT claim
- Authorization rules can now include org_type conditions:
  - `provider_admin` can manage consumer org relationships
  - `consumer_admin` cannot create orgs or modify org_type
  - `platform_admin` has full access across all org types

### 5.5 API Contract Changes

- `/orgs` POST accepts `org_type` in the request body (not a query param, not a path segment)
- org_type is always included in org object responses (read-only)
- No new query filters for org_type in generic org listing (platform service can have internal filtering)

### 5.6 What This Does NOT Do

- Does not create org-to-org relationships (that's application-layer)
- Does not create parent/child org hierarchy (orgs remain flat siblings)
- Does not change the membership model (org-to-user, not org-to-org)
- Does not change the 4-service topology

### 5.7 What This DOES Enable

- Platform service can enforce org-type-based business rules
- Authz-core can make authorization decisions based on org_type from the JWT
- Application services can trust org_type claims without additional API calls
- Consumer orgs cannot reclassify themselves — only the platform can set or change org_type

## 6. Open Questions

1. **Can org_type be changed after creation?** A provider becoming a consumer (or vice versa) may be a real scenario. Option A allows this via a platform-only endpoint, but we should decide on the business rules for reclassification.
2. **Does org_type need to be indexable for search?** The platform service may need to filter/query orgs by type for admin dashboards. This is an internal implementation concern.
3. **Should we add an org tier/plan field separately?** org_type answers "what role does this org play?" but not "what plan is this org on?" These are orthogonal dimensions and should be separate fields.
