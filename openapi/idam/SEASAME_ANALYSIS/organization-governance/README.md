# Organization Governance

> **Component:** Multi-tenant organizational structure — orgs, roles, members, invitations, SSO, SCIM, webhooks
> **Priority:** P1 — Enables B2B multi-tenant applications with organizational boundaries
> **Service:** org-mgmt (43 endpoints, 3,734 lines)

---

## The Pitch

**Buyer Question:** *Can I create organizations, invite members, assign roles, configure SSO, synchronize directories via SCIM, and manage webhooks — all through a consistent API with tenant isolation?*

If the answer is yes, you have a multi-tenant platform that can serve businesses of any size. Organization governance isn't just about creating groups — it's about the entire lifecycle of organizational entities: creation, membership management, role assignment, directory synchronization, identity federation, and event-driven extensibility. It's the bridge between individual identity (user management) and organizational identity (enterprise IAM).

---

## What This Component Does

Organization Governance manages the organizational structure and membership that underpins multi-tenant applications:

1. **Organization Lifecycle** — Create, update, delete, and retrieve organizations with custom metadata
2. **Member Management** — Add, remove, update, and query organization members with role assignment
3. **Role Management** — Create, update, delete roles within organizations with permission scoping
4. **Permission Management** — Define granular permissions within roles and manage permission hierarchies
5. **Invitation Management** — Send, track, and manage member invitations with expiration and status
6. **Application Management** — Register and manage applications within organizations (client IDs, redirect URIs)
7. **SSO Configuration** — Configure SAML 2.0 and OIDC identity providers for organization-level SSO
8. **SCIM Provisioning** — Sync organizations and members from external identity directories (Okta, Azure AD)
9. **Webhook Subscriptions** — Register event-driven webhooks for organization events (member join, role change)
10. **Directory Search** — Search organizations and members with filters and pagination

---

## Entity Model

### Organization Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | Organization identifier |
| `name` | String (255) | Yes | Organization display name |
| `domain` | String (255) | No | Primary email domain (for SSO auto-discovery) |
| `tenant_id` | UUID | Yes | Parent tenant scope |
| `logo_url` | String (1024) | No | Organization logo URL |
| `website` | String (512) | No | Organization website |
| `industry` | String (128) | No | Industry classification |
| `metadata` | JSON | No | Custom organization attributes |
| `sso_enabled` | Boolean | No | Whether SSO is enabled |
| `scim_enabled` | Boolean | No | Whether SCIM sync is enabled |
| `created_at` | DateTime | Yes | Creation timestamp |
| `updated_at` | DateTime | Yes | Last update timestamp |

### Member Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | Member identifier |
| `org_id` | UUID | Yes | Associated organization |
| `user_id` | UUID | Yes | Associated user |
| `role_id` | UUID | Yes | Assigned role |
| `status` | Enum: [active, pending, suspended] | Yes | Membership status |
| `joined_at` | DateTime | Yes | Join timestamp |
| `invited_by` | UUID | No | Inviting member |
| `last_activity` | DateTime | No | Last activity timestamp |

### Role Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | Role identifier |
| `name` | String (255) | Yes | Role name |
| `description` | String (512) | No | Role description |
| `org_id` | UUID | No | Organization scope (global if null) |
| `permissions` | Array[String] | Yes | Associated permission strings |
| `is_builtin` | Boolean | No | Predefined vs custom |
| `created_at` | DateTime | Yes | Creation timestamp |

### Invitation Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | Invitation identifier |
| `org_id` | UUID | Yes | Target organization |
| `email` | String (255) | Yes | Invitee email |
| `role_id` | UUID | Yes | Proposed role |
| `status` | Enum: [pending, accepted, declined, expired] | Yes | Invitation status |
| `invited_by` | UUID | Yes | Inviter |
| `expires_at` | DateTime | Yes | Expiration timestamp |
| `accepted_at` | DateTime | No | Acceptance timestamp |

### SSO Configuration Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | SSO configuration identifier |
| `org_id` | UUID | Yes | Associated organization |
| `provider` | Enum: [saml, oidc] | Yes | Identity provider type |
| `entity_id` | String (255) | No | SAML entity ID / OIDC issuer |
| `metadata_url` | String (1024) | No | SAML metadata URL |
| `signing_certificate` | String (2048) | No | IdP signing certificate |
| `signing_key` | String (2048) | No | SP signing key |
| `acs_url` | String (512) | No | Assertion Consumer Service URL |
| `single_logout_url` | String (512) | No | SLO URL |
| `is_active` | Boolean | Yes | Whether SSO is active |

### SCIM Configuration Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | SCIM configuration identifier |
| `org_id` | UUID | Yes | Associated organization |
| `provider` | Enum: [okta, azure_ad, generic] | Yes | Source directory |
| `api_key` | String (255) | No | SCIM API key |
| `base_url` | String (512) | No | SCIM endpoint URL |
| `last_sync` | DateTime | No | Last successful sync |
| `members_synced` | Integer | No | Number of members synced |

### Webhook Subscription Entity

| Field | Type | Required | Purpose |
|-------|------|----------|---------|
| `id` | UUID | Yes | Subscription identifier |
| `org_id` | UUID | Yes | Associated organization |
| `url` | String (1024) | Yes | Webhook endpoint URL |
| `events` | Array[String] | Yes | Subscribed event types |
| `secret` | String (255) | No | HMAC signing secret |
| `is_active` | Boolean | Yes | Whether subscription is active |
| `created_at` | DateTime | Yes | Creation timestamp |

---

## Entity Relationships

```
Organization ───┬── Member (one2many)              ← Org members
                ├── Role (one2many)                  ← Org roles
                ├── Invitation (one2many)            ← Pending invites
                ├── SSOConfiguration (one2many)      ← SSO providers
                ├── SCIMConfiguration (one2one)      ← Directory sync
                ├── Application (one2many)           ← Registered apps
                ├── WebhookSubscription (one2many)   ← Event webhooks
                └── Permission (many2many)           ← Org permissions
```

---

## Required API Endpoints

### Organization Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/orgs` | List all organizations |
| `POST` | `/api/v1/orgs` | Create a new organization |
| `GET` | `/api/v1/orgs/{id}` | Get organization details |
| `POST` | `/api/v1/orgs/{id}/update` | Update organization |
| `DELETE` | `/api/v1/orgs/{id}` | Delete organization |

### Member Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/orgs/{id}/members/add` | Add member to org |
| `POST` | `/api/v1/orgs/{id}/members/update` | Update member role |
| `POST` | `/api/v1/orgs/{id}/members/remove` | Remove member from org |
| `GET` | `/api/v1/orgs/{id}/members` | List org members |

### Role Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/orgs/{id}/roles/create` | Create a new role |
| `POST` | `/api/v1/orgs/{id}/roles/update` | Update a role |
| `DELETE` | `/api/v1/orgs/{id}/roles/delete` | Delete a role |
| `GET` | `/api/v1/orgs/{id}/roles` | List org roles |

### Permission Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/orgs/{id}/roles/permissions/add` | Add permission to role |
| `POST` | `/api/v1/orgs/{id}/roles/permissions/remove` | Remove permission from role |
| `GET` | `/api/v1/orgs/{id}/roles/{roleId}/permissions` | List role permissions |

### Invitation Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/orgs/{id}/invitations/create` | Create member invitation |
| `POST` | `/api/v1/orgs/{id}/invitations/resend` | Resend invitation |
| `POST` | `/api/v1/orgs/{id}/invitations/cancel` | Cancel invitation |
| `GET` | `/api/v1/orgs/{id}/invitations` | List invitations |

### Application Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/orgs/{id}/applications/create` | Register application |
| `POST` | `/api/v1/orgs/{id}/applications/update` | Update application |
| `DELETE` | `/api/v1/orgs/{id}/applications/delete` | Delete application |
| `GET` | `/api/v1/orgs/{id}/applications` | List org applications |

### SSO Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/orgs/{id}/sso/configure` | Configure SSO provider |
| `POST` | `/api/v1/orgs/{id}/sso/test` | Test SSO configuration |

### SCIM Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/orgs/{id}/scim/sync` | Trigger directory sync |
| `GET` | `/api/v1/orgs/{id}/scim/config` | Get SCIM configuration |
| `POST` | `/api/v1/orgs/{id}/scim/config` | Update SCIM configuration |

### Webhook Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/orgs/{id}/webhooks/create` | Create webhook subscription |
| `POST` | `/api/v1/orgs/{id}/webhooks/update` | Update webhook |
| `DELETE` | `/api/v1/orgs/{id}/webhooks/delete` | Delete webhook |
| `GET` | `/api/v1/orgs/{id}/webhooks` | List webhook subscriptions |
| `POST` | `/api/v1/orgs/{id}/webhooks/{id}/test` | Test webhook delivery |

---

## Competitive Positioning

### Where Sesame-IDAM Wins
- **API-first organization management** — Every org operation is available via REST. No admin console dependency.
- **Tenant-scoped organizations** — Organizations are automatically isolated by tenant_id.
- **Built-in SCIM** — SCIM sync for Okta/Azure AD integration is built in, not a plugin.
- **Webhook extensibility** — Event-driven webhooks for org events enable custom integrations.

### Where Sesame-IDAM Lags
- **No org hierarchy** — Organizations are flat. No parent-child org relationships.
- **No org billing** — No subscription management, usage tracking, or billing integration.
- **No org marketplace** — No app marketplace or template organizations.
- **No org analytics** — No dashboards showing org membership, activity, or compliance.

---

## Competitive Intelligence Deep Dive

### Okta: Group-Based Org Management
Okta's Groups are the foundation of org membership, with dynamic group rules, SCIM sync, and role inheritance. Okta also supports org units for hierarchical organization. **Sesame Gap:** No group rules, no org units, no hierarchical org structure.

### Auth0: Organization Roles
Auth0's organizations support role templates, invitation flows, and SSO configuration per org. **Sesame Gap:** Auth0 lacks SCIM sync and webhook subscriptions.

### PingIdentity: Enterprise Org Models
Ping supports complex org hierarchies with parent-child relationships, organizational boundaries, and policy inheritance. **Sesame Gap:** No org hierarchy or policy inheritance.

---

## Implementation Roadmap

### Phase 1: Core Org (Complete) — P1
1. Organization CRUD ✅
2. Member management (add/remove/update) ✅
3. Role management with permissions ✅
4. Invitation management ✅
5. Application registration ✅
6. SSO configuration (SAML/OIDC) ✅
7. SCIM sync (Okta, Azure AD) ✅
8. Webhook subscriptions ✅

### Phase 2: Advanced Org (Not Implemented) — P1
1. Parent-child organization hierarchy
2. Dynamic group rules based on attributes
3. Organization billing and subscription management
4. Org analytics dashboard

### Phase 3: Enterprise Features (Not Implemented) — P2
1. Organization marketplace (pre-built org templates)
2. Cross-tenant organization bridging
3. Organization compliance reporting (SOC 2, ISO 27001)
4. Organization-level audit logging

---

## Key Takeaway for Buyers

Sesame-IDAM's organization governance is **functionally complete for basic multi-tenant operations** — orgs, members, roles, invitations, SSO, SCIM, and webhooks are all implemented. The gap is in **enterprise org features**: hierarchy, billing, and analytics.

**For B2B SaaS applications needing org-scoped identity**, Sesame-IDAM is an excellent choice. **For enterprises requiring org hierarchies and compliance dashboards**, Okta or PingIdentity remain the better choice until these features are added.
