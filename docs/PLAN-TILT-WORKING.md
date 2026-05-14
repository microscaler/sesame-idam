# Plan: Get Tilt Working for Sesame-IDAM (Iterative)

## Current State

| Step | Status | Notes |
|------|--------|-------|
| `cargo check --workspace` | ✅ 0 errors | All 6 gen/impl package names match |
| `brrtrouter-gen lint` | ✅ 6/6 specs pass | authz-core + identity-user-mgmt fixed |
| `sesame-idam gen suite idam` | ✅ works | Monkey-patches applied in CLI shim |
| `sesame-idam build microservice` | ❌ FAILS | "unknown service: authz-core. Valid: idam" |
| Base Docker image | ❌ MISSING | `sesame-idam-base:latest` not built |
| Helm chart | ✅ exists | `helm/sesame-idam-microservice/` with values per service |
| Tiltfile | ✅ syntactically valid | Hardcoded service discovery, no blob parsing |
| docker/ | ✅ exists | `docker/base/Dockerfile` + `docker/microservices/Dockerfile.template` |
| k8s namespace | ✅ exists | `k8s/microservices/namespace.yaml` for `sesame-idam` |
| `cargo zigbuild` | ✅ available | `/home/casibbald/.cargo/bin/cargo-zigbuild` |
| Tilt binary | ✅ available | `/usr/local/bin/tilt` |
| Kind cluster | ✅ `kind-kind` exists | Shared cluster, namespace `sesame-idam` exists |

## What's Wrong

**The `sesame-idam build microservice <name>` command fails** with:
```
❌ unknown service: authz-core. Valid: idam
```

This happens because the build tool's service discovery (`service_to_suite()`) only recognizes the `hauliage` suite. Sesame-IDAM stores specs under `openapi/idam/<service>/`, but the tool maps `openapi/{suite}/` directories to suite names. The CLI shim monkey-patches the gen path mapping, but the **build** tool doesn't use the same monkey-patching — it goes through a different code path in `brrtrouter_tooling.build.workspace_build`.

**Root cause**: The `sesame-idam` CLI shim patches `brrtrouter_tooling.workspace.discovery.suites` (used by `gen`), but `build` uses `brrtrouter_tooling.build.workspace_build` which has its own service discovery logic that doesn't go through the monkey-patched discovery.

## Iterative Plan

### Step 0: Fix Build Tool Service Discovery (30 min)

**Goal**: `sesame-idam build microservice <name>` works for all 6 services.

**Problem**: The build tool looks for `openapi/idam/<service>/openapi.yaml` but its service discovery only recognizes `openapi/hauliage/<service>/`.

**Fix options:**

**Option A: Add `idam` as a recognized suite in `service_to_suite()`** (quickest)
- Monkey-patch `brrtrouter_tooling.workspace.discovery.services.service_to_suite()` in the CLI shim to also recognize `idam` as a valid suite
- Add `idam` to the `suites_with_bff()` return list
- Add `idam` service mapping so `service_to_suite("authz-core")` returns `"idam"`

**Option B: Create `bff-suite-config.yaml` with idam suite** (cleaner)
- The existing `openapi/idam/bff-suite-config.yaml` already lists 6 services
- Add a `suite` mapping that tells the build tool which suite each service belongs to
- The build tool needs to know that `authz-core` is under suite `idam`

**Recommended: Option A** — patch `service_to_suite()` and `suites_with_bff()` in the CLI shim to recognize `idam` as a valid suite with `openapi/idam/` as its OpenAPI directory. This is the same monkey-patching pattern already used for gen.

**Files to modify:**
- `tooling/src/sesame_idam_tooling/cli/main.py` — add `idam` suite patching alongside the existing gen monkey-patches

**Acceptance criteria:**
```bash
cd seasame-idam && sesame-idam build microservice authz-core  # exits 0
cd seasame-idam && sesame-idam build microservice identity-login-service  # exits 0
```

### Step 1: Build Base Docker Image (15 min)

**Goal**: `sesame-idam-base:latest` exists in local Docker.

**Commands:**
```bash
sesame-idam docker build-base
```

This builds `docker/base/Dockerfile` and tags it as `sesame-idam-base:latest`.

**Acceptance criteria:**
```bash
docker image inspect sesame-idam-base:latest  # exits 0
```

### Step 2: Build One Service End-to-End (30 min)

**Goal**: Single service (authz-core) builds Docker image and deploys to K8s via Tilt.

**Commands to test manually first:**
```bash
# 1. Build the binary
sesame-idam build microservice authz-core

# 2. Copy binary to artifacts
sesame-idam docker copy-binary \
  microservices/target/x86_64-unknown-linux-musl/debug/authz_core \
  build_artifacts/amd64/authz_core \
  authz_core

# 3. Build Docker image
sesame-idam docker build-image-simple \
  localhost:5001/sesame-idam-authz-core \
  docker/microservices/Dockerfile.template \
  build_artifacts/amd64/authz_core \
  build_artifacts \
  --service authz-core
```

**Acceptance criteria:**
- Binary builds: `microservices/target/x86_64-unknown-linux-musl/debug/authz_core` exists
- Docker image builds: `docker image inspect localhost:5001/sesame-idam-authz-core` exits 0
- Image runs: `docker run --rm localhost:5001/sesame-idam-authz-core --help` (or similar)

### Step 3: Fix Tiltfile Pipeline Ordering (30 min)

**Current Tiltfile issues:**
1. The Tiltfile's `create_microservice_build()` calls `sesame-idam build microservice <name>` but the build command needs `openapi/idam/<service>/openapi.yaml` which the build tool's discovery doesn't find yet (resolved in Step 0)
2. The `get_package_name()` function reads the **impl** crate's `name` field from `Cargo.toml`, which returns `sesame_idam_authz_core_gen_impl` — but the build tool expects the **gen** crate name pattern or needs a mapping

**Fix:**
- Update `get_package_name()` to return the correct binary name (from `[[bin]] name` in impl/Cargo.toml)
- Ensure the Tiltfile's `build-%s` resource depends on the gen step (already has `resource_deps`)
- Fix the `copy-%s` resource: it uses `get_package_name()` which returns the impl crate name, not the binary name. Use `[[bin]] name` from impl/Cargo.toml instead.

**Key change in Tiltfile:**
```python
def get_binary_name(name):
    """Return the binary name for a service (from [[bin]] name in impl/Cargo.toml)."""
    manifest = 'microservices/idam/%s/impl/Cargo.toml' % name
    result = str(local('grep -A2 "^\[\[bin\]\]" "%s" | grep "^name" | head -1 | sed "s/^name = *//;s/[^a-zA-Z0-9_-]//g"' % manifest, quiet=True)).strip()
    return result or name.replace('-', '_')
```

**Acceptance criteria:**
```bash
tilt lint --dry-run  # or tilt up --dry-run
# No errors in Tiltfile Starlark syntax
# All resources resolve
```

### Step 4: Run Tilt Up (20 min)

**Goal**: All 6 services start in K8s.

**Commands:**
```bash
cd seasame-idam && tilt up --port 10351 --host 0.0.0.0
```

**Expected Tilt pipeline per service:**
```
1. build-tooling          → pip install (once)
2. build-base-image       → docker build (once)
3. authz-core-lint        → brrtrouter-gen lint
4. authz-core-service-gen → sesame-idam gen suite idam --service authz-core
5. build-authz-core       → sesame-idam build microservice authz-core
6. copy-authz-core        → copy binary to build_artifacts/
7. docker-authz-core      → docker build image
8. k8s_yaml               → helm deploy
9. k8s_resource           → port forward 8102
```

**Acceptance criteria:**
- All 6 services show green in Tilt UI (port 10351)
- `kubectl get pods -n sesame-idam` shows 6 Running pods
- `kubectl get svc -n sesame-idam` shows 6 services with ports

### Step 5: Fix What Breaks (iterative)

Common issues and how to fix:

| Issue | Likely Cause | Fix |
|-------|-------------|-----|
| Tiltfile parse error | Starlark syntax | Run `tilt lint` to get line number |
| Build fails | Service discovery | Check `sesame-idam build microservice <name>` directly |
| Docker image build fails | Missing Dockerfile template vars | Check `docker build-image-simple` args |
| Helm deploy fails | Missing values or wrong Helm version | Run `helm lint helm/sesame-idam-microservice/` |
| Pod CrashLoopBackOff | Missing config/env vars | Check pod logs: `kubectl logs -n sesame-idam <pod>` |
| Port forward fails | Port already in use | `lsof -i :<port>` to find conflict |

## What This Plan Does NOT Include

These are out of scope for "getting Tilt working":

- **Phase 2**: build.rs, config/service.yaml, services layer — stubs work without these
- **Phase 3**: org_resolution.rs, tests/, seeds/ — not needed for startup
- **Phase 4**: workspace cleanup (database crate rename) — not blocking
- **Data infrastructure**: Redis, PostgreSQL — already in shared Kind cluster
- **Live update sync** — can add after first successful `tilt up`

## Risk Assessment

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Build tool service discovery doesn't patch correctly | Medium | Test `sesame-idam build microservice authz-core` directly before Tilt |
| Docker registry not configured in Kind | Medium | Use `docker image tag` instead of `localhost:5001/` if registry not set up |
| Helm chart needs environment-specific values | Medium | Start with minimal values, add env vars via configmap |
| Base Dockerfile needs musl cross-compiled binary | Low | `cargo zigbuild` already configured in build command |

## Execution Order

```
Step 0: Fix build tool service discovery (tooling/src/sesame_idam_tooling/cli/main.py)
Step 1: Build base Docker image (sesame-idam docker build-base)
Step 2: Build one service end-to-end (authz-core)
Step 3: Fix Tiltfile (binary name resolution, resource ordering)
Step 4: Run tilt up (all 6 services)
Step 5: Fix whatever breaks (iterative)
```

Total estimated time: 2-3 hours
