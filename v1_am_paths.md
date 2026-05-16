justfile:  echo "   4. Extract /api/v1/am/principals/*, /api/v1/am/authorize → openapi/authz-core/"
justfile:  echo "   5. Extract /api/v1/am/api-keys/* → openapi/api-keys/"
justfile:  echo "   6. Extract /orgs/*, /api/v1/am/applications/* → openapi/org-mgmt/"
docs/cross-repo-auth-analysis.md:- **API path**: `/api/v1/identity/*`, `/api/v1/am/*`, `/auth`
docs/cross-repo-auth-analysis.md:Three different path namespaces (`/api/v1/identity`, `/auth`, `/api/v1/am`) instead of a single, clear resource-based API.
docs/audit/security_evaluation_001.md:| `/api/v1/am/applications` | GET | `list_applications` |
docs/audit/security_evaluation_001.md:| `/api/v1/am/applications/{app_id}/roles` | GET | `list_roles` |
docs/audit/security_evaluation_001.md:| `/api/v1/am/applications/{app_id}/permissions` | GET | `list_permissions` |
docs/audit/security_evaluation_001.md:| `/api/v1/am/applications/{app_id}/roles/{role_id}/permissions` | GET | `get_role_permissions` |
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
docs/mermaid/sequence.md:    H --> I[On Save → call POST/PUT /api/v1/am/roles]
docs/mermaid/HLD.md:    SPA -->|/api/v1/am/authorize| AC
docs/mermaid/HLD.md:    Admin -->|/organizations/*, /api/v1/am/*| OM
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
docs/llmwiki/entities/entity-tenant.md:| `/api/v1/am/tenants` endpoints | Stale paths; tenant management is conceptual | Medium — no tenant CRUD endpoints |
docs/Epics/07-caching-strategy/stories/story-7.2.md:        Handler->>Authz: POST /api/v1/am/authorize {subject, org, action}
docs/Epics/04-hybrid-authz-model/hybrid.md:- **Fine-grained**: `POST /api/v1/am/authorize` with action + resource context -- ABAC rules -- Cached in Redis 30s TTL
docs/Epics/05-token-versioning/stories/story-5.4.md:    Admin->>Authz: PUT /api/v1/am/roles/{role} {new_perms}
docs/Epics/05-token-versioning/stories/story-5.1.md:    Admin->>Authz: PUT /api/v1/am/roles/{role} {new_permissions}
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
