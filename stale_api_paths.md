justfile:  echo "   4. Extract /api/v1/am/principals/*, /api/v1/am/authorize → openapi/authz-core/"
justfile:  echo "   5. Extract /api/v1/am/api-keys/* → openapi/api-keys/"
justfile:  echo "   6. Extract /orgs/*, /api/v1/am/applications/* → openapi/org-mgmt/"
README.md:| **authz-core** | `/api/v1/am/authorize`, `/api/v1/am/principal/*`, `/api/v1/am/principals/*` | EXTREME | Per-request authorization, principal/effective, role evaluation, attribute management |
README.md:| **api-keys** | `/api/v1/am/api-keys/*` | HIGH | API key lifecycle, validation (personal + org), rotation, archival |
README.md:| **org-mgmt** | `/orgs/*`, `/api/v1/am/applications/*` | LOW | Org lifecycle, memberships, SSO/SCIM, roles, permissions, applications, webhooks |
stale_api_paths.md:justfile:  echo "   4. Extract /api/v1/am/principals/*, /api/v1/am/authorize → openapi/authz-core/"
stale_api_paths.md:justfile:  echo "   5. Extract /api/v1/am/api-keys/* → openapi/api-keys/"
stale_api_paths.md:justfile:  echo "   6. Extract /orgs/*, /api/v1/am/applications/* → openapi/org-mgmt/"
stale_api_paths.md:README.md:| **authz-core** | `/api/v1/am/authorize`, `/api/v1/am/principal/*`, `/api/v1/am/principals/*` | EXTREME | Per-request authorization, principal/effective, role evaluation, attribute management |
stale_api_paths.md:README.md:| **api-keys** | `/api/v1/am/api-keys/*` | HIGH | API key lifecycle, validation (personal + org), rotation, archival |
stale_api_paths.md:README.md:| **org-mgmt** | `/orgs/*`, `/api/v1/am/applications/*` | LOW | Org lifecycle, memberships, SSO/SCIM, roles, permissions, applications, webhooks |
docs/llmwiki/topics/topic-hybrid-authz.md:| `/api/v1/am/authorize` | POST | online-only |
docs/llmwiki/topics/topic-hybrid-authz.md:| `/api/v1/am/principal/effective` | POST | online-only |
docs/llmwiki/topics/topic-api-key-validation.md:| `/api/v1/am/api-keys` | POST | Create API key |
docs/llmwiki/topics/topic-api-key-validation.md:| `/api/v1/am/api-keys/{id}` | GET/PATCH/DELETE | Manage key |
docs/llmwiki/topics/topic-api-key-validation.md:| `/api/v1/am/api-keys/validate` | POST | Validate any API key |
docs/llmwiki/topics/topic-api-key-validation.md:| `/api/v1/am/api-keys/validate/personal` | POST | Validate personal key |
docs/llmwiki/topics/topic-api-key-validation.md:| `/api/v1/am/api-keys/validate/org` | POST | Validate org-scoped key |
docs/llmwiki/topics/topic-api-key-validation.md:| `/api/v1/am/api-keys/archived` | GET | Fetch expired/revoked keys |
docs/llmwiki/topics/topic-api-key-validation.md:| `/api/v1/am/api-keys/usage` | GET | Usage statistics |
docs/llmwiki/topics/topic-api-key-validation.md:| `/api/v1/am/api-keys/import` | POST | Import from third-party |
docs/cross-repo-auth-analysis.md:- **API path**: `/api/v1/identity/*`, `/api/v1/am/*`, `/auth`
docs/cross-repo-auth-analysis.md:Three different path namespaces (`/api/v1/identity`, `/auth`, `/api/v1/am`) instead of a single, clear resource-based API.
docs/llmwiki/topics/topic-authorization-flow.md:| `/api/v1/am/authorize` | POST | online-only | authz-core | Fine-grained resource check always online |
docs/llmwiki/topics/topic-authorization-flow.md:| `/api/v1/am/principal/effective` | POST | online-only | authz-core | JWT claim enrichment, always online |
docs/llmwiki/topics/topic-authorization-flow.md:| `/api/v1/am/api-keys/validate` | POST | online-only | api-keys | Key validation needs freshness for revocation |
docs/llmwiki/topics/topic-authorization-flow.md:| `/api/v1/am/api-keys` (CRUD) | POST/PUT/DELETE | online-only | api-keys | Key creation/revocation always fresh |
docs/llmwiki/topics/topic-authorization-flow.md:identity-login-service → POST /api/v1/am/principal/effective {user_id, org_id} →
docs/audit/openapi_example_coverage.csv:org-mgmt,/api/v1/am/applications,GET,list_applications,No,No,none,
docs/audit/openapi_example_coverage.csv:org-mgmt,/api/v1/am/applications,POST,create_application,No,No,none,
docs/audit/openapi_example_coverage.csv:org-mgmt,/api/v1/am/applications/{app_id},GET,get_application,No,No,none,
docs/audit/openapi_example_coverage.csv:org-mgmt,/api/v1/am/applications/{app_id}/roles,GET,list_roles,No,200,partial,response_200:application/json
docs/audit/openapi_example_coverage.csv:org-mgmt,/api/v1/am/applications/{app_id}/roles,POST,create_role,Yes,201,full,"request:application/json, response_201:application/json"
docs/audit/openapi_example_coverage.csv:org-mgmt,/api/v1/am/applications/{app_id}/roles/{role_id},GET,get_role,No,200,partial,response_200:application/json
docs/audit/openapi_example_coverage.csv:org-mgmt,/api/v1/am/applications/{app_id}/permissions,GET,list_permissions,No,No,none,
docs/audit/openapi_example_coverage.csv:org-mgmt,/api/v1/am/applications/{app_id}/permissions,POST,create_permission,No,No,none,
docs/audit/openapi_example_coverage.csv:org-mgmt,/api/v1/am/applications/{app_id}/roles/{role_id}/permissions,GET,get_role_permissions,No,No,none,
docs/audit/openapi_example_coverage.csv:org-mgmt,/api/v1/am/applications/{app_id}/roles/{role_id}/permissions,POST,assign_permission_to_role,No,No,none,
docs/audit/openapi_example_coverage.csv:org-mgmt,/api/v1/am/applications/{app_id}/roles/{role_id}/permissions,DELETE,revoke_permission_from_role,No,No,none,
docs/audit/openapi_example_coverage.md:| GET | `/api/v1/am/applications` | `list_applications` |
docs/audit/openapi_example_coverage.md:| GET | `/api/v1/am/applications/{app_id}/roles` | `list_roles` |
docs/audit/openapi_example_coverage.md:| GET | `/api/v1/am/applications/{app_id}/permissions` | `list_permissions` |
docs/audit/openapi_example_coverage.md:| POST | `/api/v1/am/applications` | res.400.application/json |
docs/audit/openapi_example_coverage.md:| GET | `/api/v1/am/applications/{app_id}` | res.404.application/json |
docs/audit/openapi_example_coverage.md:| POST | `/api/v1/am/applications/{app_id}/roles` | res.400.application/json |
docs/audit/openapi_example_coverage.md:| GET | `/api/v1/am/applications/{app_id}/roles/{role_id}` | res.404.application/json |
docs/audit/openapi_example_coverage.md:| POST | `/api/v1/am/applications/{app_id}/permissions` | res.400.application/json |
docs/audit/openapi_example_coverage.md:| GET | `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` | res.404.application/json |
docs/audit/openapi_example_coverage.md:| POST | `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` | res.400.application/json, res.404.application/json |
docs/audit/openapi_example_coverage.md:| DELETE | `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` | res.404.application/json |
docs/audit/security_evaluation_001.md:| `/api/v1/am/applications` | GET | `list_applications` |
docs/audit/security_evaluation_001.md:| `/api/v1/am/applications/{app_id}/roles` | GET | `list_roles` |
docs/audit/security_evaluation_001.md:| `/api/v1/am/applications/{app_id}/permissions` | GET | `list_permissions` |
docs/audit/security_evaluation_001.md:| `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` | GET | `get_role_permissions` |
docs/llmwiki/entities/entity-tenant.md:| `/api/v1/am/tenants` endpoints | Stale paths; tenant management is conceptual | Medium — no tenant CRUD endpoints |
docs/service-topology-design.md:| **authz-core** | `openapi/authz-core/openapi.yaml` | `/api/v1/am/authorize`, `/api/v1/am/principal/*` | 8102 | EXTREME: called on EVERY consumer API request |
docs/service-topology-design.md:| **api-keys** | `openapi/api-keys/openapi.yaml` | `/api/v1/am/api-keys/*` | 8103 | HIGH: M2M validation = hash lookup, independently spiky |
docs/service-topology-design.md:| **org-mgmt** | `openapi/org-mgmt/openapi.yaml` | `/orgs/*`, `/api/v1/am/applications/*` | 8104 | LOW: admin-heavy, near-zero scaling pressure |
docs/service-topology-design.md:    Consumer->>AC: POST /api/v1/am/authorize<br/>Authorization: Bearer <JWT><br/>{sub, org_id, action: "invoice:write"}
docs/service-topology-design.md:| `POST /api/v1/am/authorize` | authz-core | MEDIUM (role evaluation) | Redis (30s TTL) |
docs/service-topology-design.md:| `POST /api/v1/am/api-keys/validate/personal` | api-keys | LOW (hash lookup) | Redis (no need — hash lookup is fast) |
docs/service-topology-design.md:| `POST /api/v1/am/api-keys/validate/org` | api-keys | LOW-MEDIUM (hash + org lookup) | Redis (5s TTL) |
docs/service-topology-design.md:| `POST /api/v1/am/principal/effective` | authz-core | HIGH (resolves hierarchy) |
docs/service-topology-design.md:| `GET /api/v1/am/principals/roles` | authz-core | LOW |
docs/service-topology-design.md:| `GET /api/v1/am/principals/attributes` | authz-core | LOW |
docs/service-topology-design.md:| All `POST/PUT/DELETE /api/v1/am/applications/*` | org-mgmt |
docs/PRS_SECURITY_HARDENING.md:2. Each login triggers an authz-core call: `POST /api/v1/am/principal/effective {user_id, org_id}`
docs/llmwiki/reference/ref-api-surface.md:| `/api/v1/am/applications` | GET | List applications |
docs/llmwiki/reference/ref-api-surface.md:| `/api/v1/am/applications` | POST | Register application |
docs/llmwiki/reference/ref-api-surface.md:| `/api/v1/am/applications/{app_id}` | GET | Get application by id |
docs/llmwiki/reference/ref-api-surface.md:| `/api/v1/am/applications/{app_id}/permissions` | GET | List permissions for application |
docs/llmwiki/reference/ref-api-surface.md:| `/api/v1/am/applications/{app_id}/permissions` | POST | Create permission for application |
docs/llmwiki/reference/ref-api-surface.md:| `/api/v1/am/applications/{app_id}/roles` | GET | List roles for application |
docs/llmwiki/reference/ref-api-surface.md:| `/api/v1/am/applications/{app_id}/roles` | POST | Create role for application |
docs/llmwiki/reference/ref-api-surface.md:| `/api/v1/am/applications/{app_id}/roles/{role_id}` | GET | Get role by id |
docs/llmwiki/reference/ref-api-surface.md:| `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` | DELETE | Revoke permission from role |
docs/llmwiki/reference/ref-api-surface.md:| `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` | GET | Get permissions for role |
docs/llmwiki/reference/ref-api-surface.md:| `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` | POST | Assign permission to role |
docs/llmwiki/reference/ref-api-surface.md:- `GET /api/v1/am/applications`
docs/llmwiki/reference/ref-api-surface.md:- `GET /api/v1/am/applications/{app_id}`
docs/llmwiki/reference/ref-api-surface.md:- `POST /api/v1/am/applications`
docs/llmwiki/reference/ref-api-surface.md:- `GET /api/v1/am/applications/{app_id}/permissions`
docs/llmwiki/reference/ref-api-surface.md:- `POST /api/v1/am/applications/{app_id}/permissions`
docs/llmwiki/reference/ref-api-surface.md:- `DELETE /api/v1/am/applications/{app_id}/roles/{role_id}/permissions`
docs/llmwiki/reference/ref-api-surface.md:- `GET /api/v1/am/applications/{app_id}/roles`
docs/llmwiki/reference/ref-api-surface.md:- `GET /api/v1/am/applications/{app_id}/roles/{role_id}`
docs/llmwiki/reference/ref-api-surface.md:- `GET /api/v1/am/applications/{app_id}/roles/{role_id}/permissions`
docs/llmwiki/reference/ref-api-surface.md:- `POST /api/v1/am/applications/{app_id}/roles`
docs/llmwiki/reference/ref-api-surface.md:- `POST /api/v1/am/applications/{app_id}/roles/{role_id}/permissions`
docs/llmwiki/log.md:- `POST /applications` (not `POST /api/v1/am/applications`)
docs/sesame-idam-complete.md:| **authz-core** | `openapi/authz-core/openapi.yaml` | `/api/v1/am/authorize`, `/api/v1/am/principal/*` | EXTREME (every consumer API request) | LOW (cached) | Real-time authorization checks, principal/effective resolution, role/permission evaluation |
docs/sesame-idam-complete.md:| **api-keys** | `openapi/api-keys/openapi.yaml` | `/api/v1/am/api-keys/*` | HIGH (independently spiky) | LOW (hash lookup) | API key lifecycle, validation (personal + org variants), rotation, revocation |
docs/sesame-idam-complete.md:| **org-mgmt** | `openapi/org-mgmt/openapi.yaml` | `/orgs/*`, `/api/v1/am/applications/*` | LOW (admin-heavy) | MEDIUM (CRUD + external SSO) | Org/tenant CRUD, memberships, invitations, SSO/SAML/SCIM, roles, permissions, applications, webhooks |
docs/sesame-idam-complete.md:Base path: `/api/v1/am/authorize`, `/api/v1/am/principal/*`
docs/sesame-idam-complete.md:| POST | `/api/v1/am/authorize` | Authorization check |
docs/sesame-idam-complete.md:| POST | `/api/v1/am/principal/effective` | Resolve user's effective permissions |
docs/sesame-idam-complete.md:| GET | `/api/v1/am/principals/roles` | List principal roles |
docs/sesame-idam-complete.md:| GET | `/api/v1/am/principals/attributes` | List principal attributes |
docs/sesame-idam-complete.md:Base path: `/api/v1/am/api-keys/*`
docs/sesame-idam-complete.md:| POST | `/api/v1/am/api-keys` | Create API key (M2M / service account) |
docs/sesame-idam-complete.md:| POST | `/api/v1/am/api-keys/validate/personal` | Validate personal API key |
docs/sesame-idam-complete.md:| POST | `/api/v1/am/api-keys/validate/org` | Validate org API key |
docs/sesame-idam-complete.md:| PUT | `/api/v1/am/api-keys/{id}/rotate` | Rotate API key |
docs/sesame-idam-complete.md:| DELETE | `/api/v1/am/api-keys/{id}` | Revoke API key |
docs/sesame-idam-complete.md:Base path: `/orgs/*`, `/api/v1/am/applications/*`
docs/sesame-idam-complete.md:| GET | `/api/v1/am/applications` | List applications |
docs/sesame-idam-complete.md:| GET | `/api/v1/am/roles` | List all roles (filtered by application) |
docs/sesame-idam-complete.md:| POST | `/api/v1/am/roles` | Create role |
docs/sesame-idam-complete.md:| PUT | `/api/v1/am/roles/{roleId}` | Update role |
docs/sesame-idam-complete.md:| DELETE | `/api/v1/am/roles/{roleId}` | Deactivate role |
docs/sesame-idam-complete.md:| GET | `/api/v1/am/permissions` | List all permissions |
docs/sesame-idam-complete.md:| POST | `/api/v1/am/permissions` | Create permission |
docs/sesame-idam-complete.md:| PUT | `/api/v1/am/permissions/{permissionId}` | Update permission |
docs/sesame-idam-complete.md:| DELETE | `/api/v1/am/permissions/{permissionId}` | Deactivate permission |
docs/sesame-idam-complete.md:**The key insight: the application reads the JWT and makes decisions from it. It never calls Sesame for permission checks during normal request handling.** The `/api/v1/am/authorize` endpoint is only used for admin interfaces or for checking permissions that may have changed since the last login.
docs/sesame-idam-complete.md:    Consumer->>AC: POST /api/v1/am/authorize<br/>Authorization: Bearer *** {org_id, permission: "invoice:write"}
openapi/idam/org-mgmt/README.md:- `GET /api/v1/am/applications`
openapi/idam/org-mgmt/README.md:- `GET /api/v1/am/applications/{app_id}`
openapi/idam/org-mgmt/README.md:- `POST /api/v1/am/applications`
openapi/idam/org-mgmt/README.md:- `GET /api/v1/am/applications/{app_id}/permissions`
openapi/idam/org-mgmt/README.md:- `POST /api/v1/am/applications/{app_id}/permissions`
openapi/idam/org-mgmt/README.md:- `DELETE /api/v1/am/applications/{app_id}/roles/{role_id}/permissions`
openapi/idam/org-mgmt/README.md:- `GET /api/v1/am/applications/{app_id}/roles`
openapi/idam/org-mgmt/README.md:- `GET /api/v1/am/applications/{app_id}/roles/{role_id}`
openapi/idam/org-mgmt/README.md:- `GET /api/v1/am/applications/{app_id}/roles/{role_id}/permissions`
openapi/idam/org-mgmt/README.md:- `POST /api/v1/am/applications/{app_id}/roles`
openapi/idam/org-mgmt/README.md:- `POST /api/v1/am/applications/{app_id}/roles/{role_id}/permissions`
docs/design-doc.md:| **authz-core** | `/api/v1/am/authorize`, `/api/v1/am/principal/*` | EXTREME | Low-Medium (cache hit, role evaluation) | Scales with every authenticated request — highest volume |
docs/design-doc.md:| **api-keys** | `/api/v1/am/api-keys/*` | MEDIUM | Low-Medium (DB validation, cache) | Scales with M2M service traffic |
docs/design-doc.md:| **org-mgmt** | `/orgs/*`, `/api/v1/am/applications/*`, SCIM, webhooks | LOW | High (complex org operations, external SSO) | Scales with org lifecycle events — low volume, high complexity |
docs/design-doc.md:**Base paths:** `/api/v1/am/authorize`, `/api/v1/am/principal/*`
docs/design-doc.md:| `/api/v1/am/authorize` | POST | Per-request authorization check |
docs/design-doc.md:| `/api/v1/am/principal/effective` | POST | Resolve user's effective roles + permissions |
docs/design-doc.md:| `/api/v1/am/principals/roles` | POST | Assign/revoke principal roles |
docs/design-doc.md:| `/api/v1/am/principals/attributes` | POST | Set principal attributes (ABAC) |
docs/design-doc.md:**Base path:** `/api/v1/am/api-keys/*`
docs/design-doc.md:| `/api/v1/am/api-keys` | POST | Create API key |
docs/design-doc.md:| `/api/v1/am/api-keys/{id}` | GET/PATCH/DELETE | Manage key |
docs/design-doc.md:| `/api/v1/am/api-keys/validate` | POST | Validate any API key |
docs/design-doc.md:| `/api/v1/am/api-keys/validate/personal` | POST | Validate personal (user-scoped) key |
docs/design-doc.md:| `/api/v1/am/api-keys/validate/org` | POST | Validate org-scoped key |
docs/design-doc.md:| `/api/v1/am/api-keys/archived` | GET | Fetch expired/revoked keys |
docs/design-doc.md:| `/api/v1/am/api-keys/usage` | GET | Fetch usage statistics |
docs/design-doc.md:| `/api/v1/am/api-keys/import` | POST | Import keys from third-party systems |
docs/design-doc.md:**Base paths:** `/orgs/*`, `/api/v1/am/applications/*`
docs/design-doc.md:| **Applications** | `POST/GET /api/v1/am/applications`, roles/permissions per application | LOW |
docs/design-doc.md:    Consumer->>AC: POST /api/v1/am/authorize<br/>Authorization: Bearer *** {org_id, action: \"invoice:write\"}
docs/Epics/05-token-versioning/stories/story-5.4.md:    Admin->>Authz: PUT /api/v1/am/roles/{role} {new_perms}
docs/Epics/05-token-versioning/stories/story-5.1.md:    Admin->>Authz: PUT /api/v1/am/roles/{role} {new_permissions}
docs/mermaid/sequence.md:    H --> I[On Save → call POST/PUT /api/v1/am/roles]
docs/mermaid/HLD.md:    SPA -->|/api/v1/am/authorize| AC
docs/mermaid/HLD.md:    SPA -->|/api/v1/am/api-keys/*| AK
docs/mermaid/HLD.md:    Admin -->|/orgs/*, /api/v1/am/*| OM
docs/mermaid/HLD.md:| `openapi/authz-core/openapi.yaml` | `/api/v1/am/authorize`, `/api/v1/am/principal/*` | EXTREME | LOW (Redis cached) |
docs/mermaid/HLD.md:| `openapi/api-keys/openapi.yaml` | `/api/v1/am/api-keys/*` | HIGH | LOW (hash lookup) |
docs/mermaid/HLD.md:| `openapi/org-mgmt/openapi.yaml` | `/orgs/*`, `/api/v1/am/applications/*` | LOW | MEDIUM (CRUD) |
microservices/idam/README.md:| **authz-core** | `openapi/authz-core/openapi.yaml` | `/api/v1/am/authorize`, `/api/v1/am/principal/*` | 8002 | EXTREME | LOW | Per-request authorization checks, principal/effective resolution, role/permission evaluation |
microservices/idam/README.md:| **api-keys** | `openapi/api-keys/openapi.yaml` | `/api/v1/am/api-keys/*` | 8003 | HIGH | LOW | API key lifecycle, validation (personal + org variants), rotation, revocation |
microservices/idam/README.md:| **org-mgmt** | `openapi/org-mgmt/openapi.yaml` | `/orgs/*`, `/api/v1/am/applications/*` | 8004 | LOW | MEDIUM | Org/tenant CRUD, memberships, invitations, SSO/SAML/SCIM, roles, permissions, applications, webhooks |
openapi/README.md:| **authz-core** | `openapi/authz-core/` | `openapi.yaml` | `/api/v1/am/authorize`, `/api/v1/am/principal/*` | 8102 |
openapi/README.md:| **api-keys** | `openapi/api-keys/` | `openapi.yaml` | `/api/v1/am/api-keys/*` | 8103 |
openapi/README.md:| **org-mgmt** | `openapi/org-mgmt/` | `openapi.yaml` | `/orgs/*`, `/api/v1/am/applications/*` | 8104 |
docs/Epics/04-hybrid-authz-model/hybrid.md:- **Fine-grained**: `POST /api/v1/am/authorize` with action + resource context -- ABAC rules -- Cached in Redis 30s TTL
docs/Epics/04-hybrid-authz-model/stories/story-4.1.md:| `/api/v1/am/authorize` | POST | authz-core | Fine-grained resource check always requires online evaluation |
docs/Epics/04-hybrid-authz-model/stories/story-4.1.md:| `/api/v1/am/principal/effective` | POST | authz-core | JWT claim enrichment, always online |
docs/Epics/04-hybrid-authz-model/stories/story-4.1.md:| `/api/v1/am/principals/roles` | POST | authz-core | Role management, always online |
docs/Epics/04-hybrid-authz-model/stories/story-4.1.md:| `/api/v1/am/principals/attributes` | POST | authz-core | ABAC attributes, always online |
docs/Epics/04-hybrid-authz-model/stories/story-4.1.md:| `/api/v1/am/api-keys/validate` | POST | api-keys | Key validation always needs freshness for revocation |
docs/Epics/04-hybrid-authz-model/stories/story-4.1.md:| `/api/v1/am/api-keys/validate/personal` | POST | api-keys | Personal key validation |
docs/Epics/04-hybrid-authz-model/stories/story-4.1.md:| `/api/v1/am/api-keys/validate/org` | POST | api-keys | Org key validation |
docs/Epics/04-hybrid-authz-model/stories/story-4.1.md:| `/api/v1/am/api-keys` | POST | api-keys | Key creation, high-sensitivity |
docs/Epics/04-hybrid-authz-model/stories/story-4.1.md:| `/api/v1/am/api-keys/{id}` | DELETE | api-keys | Key revocation, always fresh |
docs/Epics/04-hybrid-authz-model/stories/story-4.1.md:| `/api/v1/am/api-keys/{id}/rotate` | PUT | api-keys | Key rotation, always fresh |
docs/Epics/04-hybrid-authz-model/stories/story-4.1.md:| `/api/v1/am/roles` | POST, PUT, DELETE | org-mgmt | Role management, always online |
docs/Epics/04-hybrid-authz-model/stories/story-4.1.md:| `/api/v1/am/permissions` | POST, PUT, DELETE | org-mgmt | Permission management, always online |
docs/Epics/04-hybrid-authz-model/stories/story-4.1.md:  - path: "/api/v1/am/authorize"
docs/Epics/04-hybrid-authz-model/stories/story-4.1.md:   - `/api/v1/am/api-keys/{id}/rotate` (should be `online-only`)
docs/Epics/04-hybrid-authz-model/stories/story-4.1.md:   - `/api/v1/am/authorize` (should be `online-only`)
docs/Epics/04-hybrid-authz-model/stories/story-4.1.md:3. Attacker changes `/api/v1/am/api-keys/{id}/rotate` from `online-only` to `jwt-only`
docs/Epics/04-hybrid-authz-model/stories/story-4.1.md:   - path: "/api/v1/am/api-keys"
docs/Epics/04-hybrid-authz-model/stories/story-4.1.md:   - path: "/api/v1/am/api-keys"
docs/Epics/04-hybrid-authz-model/stories/story-4.3.md:            Handler->>Authz: POST /api/v1/am/authorize {org_id, action}
docs/Epics/04-hybrid-authz-model/stories/story-4.3.md:- `/api/v1/am/authorize` endpoint: Document the Redis cache behavior in the endpoint description
docs/Epics/07-caching-strategy/stories/story-7.2.md:        Handler->>Authz: POST /api/v1/am/authorize {subject, org, action}
