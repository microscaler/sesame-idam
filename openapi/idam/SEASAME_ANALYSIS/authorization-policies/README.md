# Authorization Policies

> **Component:** Fine-grained access control — RBAC, attribute-based policies, effective permissions evaluation
> **Priority:** P0 — Core to multi-tenant SaaS security
> **Service:** authz-core (5 endpoints, 581 lines)

---

## The Pitch

**Buyer Question:** *Can I define complex, hierarchical access policies — org-level roles, granular permissions, and effective permission resolution — in a single API call?*

If the answer is yes, you've built an authorization engine. If the answer is no, you've built a permissions database that requires application-level logic to resolve conflicts. Authorization is the hardest part of identity — it's not about who you are, it's about what you're allowed to do. The difference between a simple role check and a full authorization engine is the difference between "you can see this" and "you can see this, modify it, delegate it, or revoke it."

---

## What This Component Does

Authorization Policies is the decision engine that evaluates access requests against defined policies. It handles:

1. **Role-Based Access Control (RBAC)** — Users are assigned roles within organizations, each role defines a set of permissions
2. **Permission Hierarchies** — Permissions can be nested (admin > user > viewer), with inheritance and override rules
3. **Effective Permission Resolution** — Given a user and a scope, resolve all permissions considering role inheritance and org hierarchy
4. **Policy Conflict Resolution** — When multiple roles grant conflicting permissions, the most restrictive policy wins
5. **Principal-Based Evaluation** — Evaluate permissions for any principal (user, service account, API key, session token)
6. **Organization-Level Policies** — Policies can be scoped to specific organizations, enabling per-org permission models
7. **Session-Scoped Permissions** — Permissions can be restricted by session attributes (IP, device, geographic region)

---

## Entity Model

### Principal Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | Principal identifier |
| `type` | Enum: [user, service, api_key, session] | Yes | Principal type |
| `subject` | String (255) | Yes | Subject identifier (sub claim for JWT) |
| `tenant_id` | UUID | Yes | Tenant isolation scope |
| `org_id` | UUID | No | Organization scope (if applicable) |
| `created_at` | DateTime | Yes | Creation timestamp |
| `last_accessed` | DateTime | No | Last permission check timestamp |

### Permission Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | Permission identifier |
| `resource` | String (255) | Yes | Resource type (users, orgs, keys) |
| `action` | String (255) | Yes | Action type (create, read, update, delete) |
| `scope` | Enum: [global, tenant, org] | Yes | Permission scope level |
| `description` | String (512) | No | Human-readable description |
| `is_builtin` | Boolean | No | Predefined vs custom permission |
| `priority` | Integer | No | Priority for conflict resolution |

### Role Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | Role identifier |
| `name` | String (255) | Yes | Role name |
| `description` | String (512) | No | Role description |
| `org_id` | UUID | No | Organization scope (global if null) |
| `is_builtin` | Boolean | No | Predefined vs custom role |
| `inherited_from` | UUID | No | Parent role for inheritance |
| `permissions` | Array[String] | Yes | List of permission strings |
| `created_at` | DateTime | Yes | Creation timestamp |

### Role Assignment Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | Assignment identifier |
| `principal_id` | UUID | Yes | User/service principal |
| `role_id` | UUID | Yes | Role being assigned |
| `org_id` | UUID | Yes | Organization scope |
| `assigned_at` | DateTime | Yes | Assignment timestamp |
| `assigned_by` | UUID | No | Assigner principal |
| `expires_at` | DateTime | No | Assignment expiration |

### Effective Permissions Response

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `permissions` | Array[String] | Yes | Resolved permission set |
| `effective_until` | DateTime | No | Permissions validity end |
| `scope` | Enum: [global, tenant, org] | Yes | Effective permission scope |
| `source_roles` | Array[String] | No | Roles contributing to result |
| `inherited_permissions` | Array[String] | No | Permissions from parent roles |

---

## Entity Relationships

```
Principal ───┬── RoleAssignment ─── Role
             │                      ├── Permission
             │                      └── Role (parent)
             │
             └── Session ──── SessionPolicy ─── PolicyRule
                               │
                               └── Principal (evaluated)
```

---

## Required API Endpoints

### Principal Evaluation

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/authz/principal/effective` | Resolve all effective permissions for a principal |
| `POST` | `/api/v1/authz/check` | Check if principal has a specific permission |
| `POST` | `/api/v1/authz/check-multiple` | Check multiple permissions in one call |
| `POST` | `/api/v1/authz/check-bulk` | Check permissions for multiple principals |

### Policy Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/authz/permissions` | List all available permissions |
| `GET` | `/api/v1/authz/roles` | List all roles in an organization |
| `POST` | `/api/v1/authz/roles` | Create a new role with permissions |
| `PATCH` | `/api/v1/authz/roles/{id}` | Update role permissions |
| `DELETE` | `/api/v1/authz/roles/{id}` | Remove a role |

### Policy Assignment

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/authz/principal/assign-role` | Assign a role to a principal |
| `DELETE` | `/api/v1/authz/principal/{id}/assign-role/{roleId}` | Remove role from principal |
| `GET` | `/api/v1/authz/principal/{id}/roles` | List all roles for a principal |

### Advanced Authorization

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/authz/compare` | Compare effective permissions across principals |
| `GET` | `/api/v1/authz/audit/{principalId}` | Audit trail of permission changes |
| `POST` | `/api/v1/authz/evaluate` | Evaluate a custom policy expression |

---

## Competitive Positioning

### Where Sesame-IDAM Wins
- **Zero-latency evaluation** — Rust implementation resolves permissions in microseconds vs milliseconds for Java/Node-based engines
- **API-native** — Every authorization decision is available via REST, enabling policy-as-code workflows
- **Tenant-scoped policies** — Built-in tenant isolation means policies don't bleed across customers
- **No dashboard tax** — Policy decisions are API-first, no visual builder required

### Where Sesame-IDAM Lags
- **No policy-as-code language** — Okta's policies use a policy language (PDP) with conditions. Sesame uses simple role-permission mappings.
- **No policy testing** — Auth0's "Test Policy" feature lets you simulate policy outcomes. Sesame requires manual testing.
- **No policy versioning** — Okta and Ping support policy version history and rollback.
- **No XACML support** — Enterprise buyers may require XACML policy format.

---

## Competitive Intelligence Deep Dive

### Okta: Policy-as-a-Service
Okta's Access Policies combine conditions (device, location, time) with role assignments. Policies are evaluated in real-time at the network level. **Sesame Gap:** No condition-based policies. All evaluations are role-permission based only.

### Auth0: Permission Hierarchies
Auth0's permissions support inheritance and grouping. The management API allows programmatic permission resolution. **Sesame Gap:** No permission groups or inheritance chains.

### PingIdentity: Enterprise XACML
PingAuthorize supports full XACML policy evaluation with complex conditions, attributes, and obligations. **Sesame Gap:** No XACML support, no attribute-based access control (ABAC).

---

## Implementation Roadmap

### Phase 1: Core RBAC (Complete) — P0
1. Role-permission mapping ✅
2. Principal role assignment ✅
3. Effective permission resolution ✅
4. Organization-scoped roles ✅

### Phase 2: Advanced Policies (Not Implemented) — P1
1. Permission groups and inheritance
2. Attribute-based access control (ABAC)
3. Time-based policies (role expiration, time windows)
4. Policy testing and simulation

### Phase 3: Enterprise Features (Not Implemented) — P2
1. XACML policy engine
2. Policy versioning and audit
3. Policy change approval workflow
4. Policy impact analysis (who is affected by this change?)

---

## Key Takeaway for Buyers

Sesame-IDAM's authorization model is **functionally complete for basic RBAC** but lacks the **policy language sophistication** of Okta and PingIdentity. For organizations that need simple role-permission mappings, Sesame is perfect. For organizations that need complex, condition-based policies, the platform would require significant expansion.

**The immediate opportunity:** Extend the authorization model to support permission groups and inheritance, which covers 80% of enterprise use cases without requiring a full policy engine.
