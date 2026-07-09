---
title: Tiltfile Architecture
status: verified
updated: 2026-05-15
sources: [PRD-SEASAME-AUDIT-REMEDIATION.md, hauliage Tiltfile, Tiltfile, actual build_image_simple CLI]
---

# Tiltfile Architecture

## Current Status: ✅ Written and Validated

The Tiltfile has been rewritten (~320 lines) following hauliage patterns, adapted for sesame-idam's nested `idam/` layout. The critical `build-image-simple` CLI args have been corrected.

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Hardcoded service discovery | Avoids Tilt Starlark `read_file()` blob parsing issues |
| `openapi/idam/<service>/` paths | Matches actual spec layout (not `openapi/<service>/`) |
| No `namespace=` in `k8s_yaml()` | Inherited from shared cluster context |
| No data infra deployment | PostgreSQL/Redis managed by shared-kind-cluster |
| sesame-idam CLI shim for gen/build | Monkey-patched shim routes to `openapi/idam/` layout |
| `build-image-simple` with correct args | Fixed from `<image> <hash_path> <artifact> --system --module` to `<image> <dockerfile_template> <hash_path> <artifact> --service` |
| `custom_build` with `live_update` | Enables hot-reload for development |

### Build Pipeline Per Service

```
1. <service>-lint        → brrtrouter-gen lint --spec ./openapi/idam/<svc>
2. <service>-service-gen → sesame-idam gen suite idam --service <svc>
3. build-<service>       → sesame-idam build microservice <svc>
4. copy-<service>        → copy binary to build_artifacts/
5. docker-<service>      → docker build from template (custom_build + live_update)
6. k8s_yaml(helm())      → Helm deployment
7. k8s_resource()        → port forward + labels
```

### Service Configuration

All services listen on ClusterIP **:8080** in-cluster (`SERVICE_HTTP_PORT`).
Optional Tilt host port-forwards: login `8101:8080`, session `8105:8080`.

| Service | In-cluster port | Gen Package |
|---------|-----------------|-------------|
| identity-login-service | 8080 | sesame_idam_identity_login_service_gen |
| identity-session-service | 8080 | sesame_idam_identity_session_service_gen |
| identity-user-mgmt-service | 8080 | sesame_idam_identity_user_mgmt_service_gen |
| authz-core | 8080 | sesame_idam_authz_core_gen |
| api-keys | 8080 | sesame_idam_api_keys_gen |
| org-mgmt | 8080 | sesame_idam_org_mgmt_gen |

### Build-image-simple CLI Fix (Critical)

**Before (broken):**
```bash
sesame-idam docker build-image-simple <image> <hash_path> <artifact_path> \
  --system idam --module <svc> --port <port> --binary-name <pkg_name>
```

**After (correct):**
```bash
sesame-idam docker build-image-simple <image> <dockerfile_template> <hash_path> <artifact_path> --service <svc>
```

The old call was missing the dockerfile template argument (pos 2), using wrong flags (`--system/--module/--port/--binary-name` instead of `--service`), and artifacts should be in `build_artifacts/<arch>/<binary_name>`.

### Tiltfile Structure

- **Lines 1-45**: Configuration (cluster context, namespace, paths, ports)
- **Lines 47-90**: Tooling build (`build-tooling`)
- **Lines 92-104**: Base Docker image (`build-base-image`)
- **Lines 106-165**: Service definitions + helper functions (`get_package_name`, `get_service_port`, `create_microservice_lint/gen/build/deployment`)
- **Lines 167-183**: Data infrastructure note (delegated to shared cluster)
- **Lines 185-190**: Per-service resource registration (loop over 6 services)
- **Lines 192-265**: `create_microservice_deployment()` — includes `custom_build` with `live_update`, correct `build-image-simple` args, architecture detection, hash generation

### What's Missing (Post-Phase 0)

| Item | Status | Notes |
|------|--------|-------|
| `tilt trigger docker-<svc>` validation | ⏳ Pending | Tilt service managed by systemd, use `systemctl --user status tilt-sesame-idam.service` |
| `sesame_idam_database` rename | ⏳ Pending | Phase 4 task |
| Database secrets/configmaps | ⏳ Pending | Phase 5 task |
| Redis K8s manifest | ⏳ Pending | Phase 5 task |
| `config/service.yaml` per service | ⏳ Pending | Phase 2 |
| `services/` layer | ⏳ Pending | Phase 2 |

## Code Anchors

- `PRD-SEASAME-AUDIT-REMEDIATION.md Section 6` — Full Tilt & tooling architecture
- `PRD-SEASAME-AUDIT-REMEDIATION.md Section 6.7` — Tiltfile rewrite plan
- `brrtrouter-workspace-architecture` skill — Tiltfile generation patterns
- `brrtrouter_tooling/docker.py` — `build_image_simple` CLI signature reference
- `tilt` skill — Tiltfile patterns, pitf...[truncated]