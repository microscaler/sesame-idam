---
title: Tiltfile Architecture
status: verified
updated: 2026-05-14
sources: [PRD-SEASAME-AUDIT-REMEDIATION.md, hauliage Tiltfile, Tiltfile]
---

# Tiltfile Architecture

## Current Status: ✅ Written and Validated

The Tiltfile has been rewritten (305 lines) following hauliage patterns, adapted for sesame-idam's nested `idam/` layout.

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Hardcoded service discovery | Avoids Tilt Starlark `read_file()` blob parsing issues |
| `openapi/idam/<service>/` paths | Matches actual spec layout (not `openapi/<service>/`) |
| No `namespace=` in `k8s_yaml()` | Inherited from shared cluster context |
| No data infra deployment | PostgreSQL/Redis managed by shared-kind-cluster |
| sesame-idam CLI shim for gen/build | Monkey-patched shim routes to `openapi/idam/` layout |

### Build Pipeline Per Service

```
1. <service>-lint        → brrtrouter-gen lint --spec ./openapi/idam/<svc>
2. <service>-service-gen → sesame-idam gen suite idam --service <svc>
3. build-<service>       → sesame-idam build microservice <svc>
4. copy-<service>        → copy binary to build_artifacts/
5. docker-<service>      → docker build from template
6. k8s_yaml(helm())      → Helm deployment
7. k8s_resource()        → port forward + labels
```

### Service Configuration

| Service | Port | Gen Package |
|---------|------|-------------|
| identity-login-service | 8101 | identity_login_service_service_api |
| identity-session-service | 8105 | identity_session_service_service_api |
| identity-user-mgmt-service | 8106 | identity_user_mgmt_service_service_api |
| authz-core | 8102 | authz_core_service_api |
| api-keys | 8103 | api_keys_service_api |
| org-mgmt | 8104 | org_mgmt_service_api |

### Tiltfile Structure

- **Lines 1-42**: Configuration (cluster context, namespace, paths, ports)
- **Lines 44-88**: Tooling build (`build-tooling`)
- **Lines 90-102**: Base Docker image (`build-base-image`)
- **Lines 104-162**: Service definitions + helper functions (`get_package_name`, `get_service_port`, `create_microservice_lint/gen/build/deployment`)
- **Lines 164-180**: Data infrastructure note (delegated to shared cluster)
- **Lines 182-187**: Per-service resource registration (loop over 6 services)

### What's Missing (Post-Phase 0)

| Item | Status | Notes |
|------|--------|-------|
| `tilt up` validation | ⏳ Pending | Needs shared-kind-cluster context |
| `sesame_idam_database` rename | ⏳ Pending | Phase 4 task |
| Database secrets/configmaps | ⏳ Pending | Phase 5 task |
| Redis K8s manifest | ⏳ Pending | Phase 5 task |

## Code Anchors

- `PRD-SEASAME-AUDIT-REMEDIATION.md Section 6` — Full Tilt & tooling architecture
- `PRD-SEASAME-AUDIT-REMEDIATION.md Section 6.7` — Tiltfile rewrite plan
- `brrtrouter-workspace-architecture` skill — Tiltfile generation patterns
