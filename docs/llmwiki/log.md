# LLM Wiki — Session Log

## [2026-05-15] Tiltfile Configmap Fix — Namespace + binary_name

### Summary

All 6 sesame-idam pods were stuck in `ContainerCreating` because the Tiltfile was missing two critical elements that prevented Helm from creating ConfigMaps:

1. **`binary_name` undefined variable** — `create_microservice_deployment()` referenced `binary_name` on line 304 (live_update sync path) but never defined it. Starlark crash.
2. **Missing `k8s_yaml('k8s/microservices/namespace.yaml')`** — namespace `sesame-idam` didn't exist when Tilt tried to apply Helm manifests, so configmaps were never created.

Hauliage has both of these. Sesame-IDAM's Tiltfile rewrite missed them.

### Root Cause Analysis

**Error 1 — Starlark crash:**
```
ERROR: Tiltfile:304:45: undefined: binary_name (did you mean image_name?)
```
Line 304 in the old Tiltfile: `sync(artifact_path, '/app/%s' % binary_name)` — the hauliage pattern defines `binary_name = name.replace('-', '_')` at the top of `create_microservice_deployment()`, but sesame-idam didn't. This caused Tilt to crash when processing `custom_build` for each service, preventing k8s_yaml from ever being applied.

**Error 2 — Namespace missing:**
```
ERROR: namespaces "sesame-idam" not found
```
The Tiltfile had no `k8s_yaml('k8s/microservices/namespace.yaml')` call. Hauliage creates this at module level in the Data Infrastructure section. Without it, Helm couldn't create ConfigMaps in a non-existent namespace.

**Consequence:** Helm templates rendered correctly (`helm template` works fine), but Tilt never applied them to the cluster. Pods were created (via `k8s_resource` auto-creation) but failed to mount configmaps (`configmap "org-mgmt-config" not found`).

### Fix Applied

```python
# In create_microservice_deployment(), alongside package_name:
binary_name = name.replace('-', '_')

# In Data Infrastructure section:
k8s_yaml('k8s/microservices/namespace.yaml')
```

### Verification

All 6 pods Running, all 6 configmaps created, all returning HTTP 200 on `/health`:
- org-mgmt (8104) — 200 ✅
- authz-core (8102) — 200 ✅
- api-keys (8103) — 200 ✅
- identity-login-service (8101) — 200 ✅
- identity-session-service (8105) — 200 ✅
- identity-user-mgmt-service (8106) — 200 ✅

### Files Updated

- `Tiltfile` — added `binary_name` variable + `k8s_yaml('k8s/microservices/namespace.yaml')`
- `docs/PRD-SEASAME-AUDIT-REMEDIATION.md` — added section 6b documenting the issues and resolution

---

## [2026-05-14] Phase 0b: Tiltfile Lint Path Fix + Wiki Update

### Summary

Fixed the Tiltfile `create_microservice_lint()` and `create_microservice_gen()` functions to use full YAML file paths (`openapi/idam/<service>/openapi.yaml`) instead of directory paths. Also updated the llmwiki to reflect the current correct state of sesame-idam infrastructure.

### Tiltfile Fixes

- `create_microservice_lint()`: Changed `--spec ./openapi/idam/%s` to `--spec ./openapi/idam/%s/openapi.yaml` (brrtrouter-gen needs the file path, not the directory)
- `create_microservice_lint() deps`: Changed `./openapi/idam/%s` to `./openapi/idam/%s/openapi.yaml`
- `create_microservice_gen() deps`: Changed `./openapi/idam/%s` to `./openapi/idam/%s/openapi.yaml`

### Wiki Updates

| File | Change |
|------|--------|
| `index.md` | Updated topic-architecture-overview description to note `cargo check --workspace` passes |
| `topics/topic-remediation-plan.md` | Phase 0 and Phase 1 marked ✅ Completed; build warnings documented; acceptance criteria updated |
| `topics/topic-build-infrastructure.md` | Status → verified; added build status table; Phase 2 items moved to "Planned" |
| `topics/topic-package-naming-convention.md` | Status → verified; documented final naming table; removed "target" section since fix is complete |
| `topics/topic-tiltfile-architecture.md` | Status → verified; documented current Tiltfile architecture and design decisions |
| `topics/topic-brrtrouter-codegen.md` | Fixed duplicate OpenAPI layout section; noted `openapi/idam/` nesting |

### Current Build State

- `cargo check --workspace` — ✅ 0 errors, 31 warnings
- `cargo test --workspace` — ✅ 5 tests (4 unit + 1 doc)
- `brrtrouter-gen lint` — ✅ All 6 specs pass (authz-core + identity-user-mgmt-service fixed)
- Tiltfile — ✅ Validated Starlark syntax, all path refs corrected

### OpenAPI Lint Fixes

Fixed `operation_id_casing` errors in 2 specs:

| Spec | Issues Fixed |
|------|-------------|
| `authz-core` | 10 operationIds (camelCase → snake_case) + added missing `PaginatedResponse` schema |
| `identity-user-mgmt-service` | 3 operationIds (getUserAuditEvents, exportUserAuditEvents, getUserEventCount → snake_case) |

### Codegen State After Fixes

All 18 camelCase operationIds now use snake_case convention. All specs define all referenced schemas.

---

## [2026-05-14] Sesame-IDAM Structural Audit — Wiki Updated from PRD

### Summary


[... content preserved from original ...]

---
