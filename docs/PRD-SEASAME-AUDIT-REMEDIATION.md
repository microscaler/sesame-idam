# PRD: Sesame-IDAM Structural Audit & Remediation

**Created:** 2026-05-14
**Author:** Agent review (comparing against Hauliage reference)
**Status:** COMPLETED — all Phases 0–4 implemented and verified
**Reference:** Hauliage workspace at `../hauliage/microservices/`

---

## 1. EXECUTIVE SUMMARY

Sesame-IDAM consists of 6 microservices with generated code (`gen/`) and binary implementations (`impl/`), plus a shared database crate (`sesame_idam_database`) and audit crate (`sesame-audit`). The workspace compiles and tests pass (`cargo check --workspace` succeeds, `cargo test --workspace` passes 4/1).

However, `brrtrouter client build` fails across all services due to package naming mismatches between `gen` and `impl` crates. Additionally, the workspace lacks several structural conventions established by Hauliage that are needed for production readiness (build scripts, config files, services layer, tests, etc.).

This PRD documents the current state, identifies all gaps against Hauliage conventions, and defines the remediation plan.

---

## 2. CURRENT STATE (WHAT WE HAVE)

### 2.1 Architecture Overview

```
seasame-idam/
├── microservices/                    ← Workspace root
│   ├── Cargo.toml                    ← 12 workspace members
│   ├── database/                     ← PooledLifeExecutor, Lifeguard pool
│   └── idam/
│       ├── api-keys/
│       │   ├── gen/                  ← BRRTRouter-generated types/handlers
│       │   └── impl/                 ← Binary + controllers + models
│       ├── authz-core/
│       │   ├── gen/
│       │   └── impl/
│       ├── identity-login-service/
│       │   ├── gen/
│       │   └── impl/
│       ├── identity-session-service/
│       │   ├── gen/
│       │   └── impl/
│       ├── identity-user-mgmt-service/
│       │   ├── gen/
│       │   └── impl/
│       └── org-mgmt/
│           ├── gen/
│           └── impl/
├── sesame-audit/                     ← HMAC-signed audit event crate
├── openapi/idam/{service}/           ← OpenAPI specs (6 services)
└── docs/llmwiki/                     ← Architecture docs
```

### 2.2 Endpoints & Services

| Service | Port | Endpoints | Access Pattern |
|---------|------|-----------|----------------|
| identity-login-service | 8101 | 20 | HIGH — login, register, OAuth, OTP |
| identity-session-service | 8105 | 13 | HIGH — refresh, OIDC, JWKS |
| identity-user-mgmt-service | 8106 | 25 | MEDIUM — user CRUD, MFA |
| authz-core | 8102 | 4 | EXTREME — principal/effective |
| api-keys | 8103 | 10 | HIGH — key validation |
| org-mgmt | 8104 | 34 | LOW — org lifecycle |
| **Total** | | **133** | |

### 2.3 Current Test Results

```
cargo test --workspace: PASS
  └── sesame_idam_database: 0 tests (compiles)
  └── sesame_audit: 4 tests PASS (event creation, HMAC signing, consistency, serialization)
  └── sesame_audit: 1 doc test PASS
  └── All other crates: 0 tests (stub stage — no business logic yet)
```

### 2.4 Build Warnings

- 26 `non_snake_case` warnings in generated modules (authz-core, identity-user-mgmt) — expected from OpenAPI endpoint names (e.g., `listAuditEvents` → `list_audit_events`). Not a blocker.
- 5 `dead_code` warnings for `EMITTER` static in `impl/src/audit.rs` — expected, stub controllers don't call audit yet.

### 2.5 What's Already Working

- [x] 6 OpenAPI specs with `X-Tenant-ID` header on all 131/133 operational endpoints
- [x] 47 entity model files across 6 services (Lifeguard ORM derive macros)
- [x] `sesame_idam_database` crate with `PooledLifeExecutor` pattern
- [x] `sesame-audit` crate with HMAC event signing
- [x] Workspace compiles: `cargo check --workspace` — 0 errors
- [x] Tests pass: 4 unit tests, 1 doc test
- [x] Stub controllers for all 133 endpoints (placeholder responses)

---

## 3. GAPS IDENTIFIED (WHAT WE DON'T HAVE)

### 3.1 CRITICAL — Build Failure

**Problem:** `brrtrouter client build` fails across all 6 services.

**Root Cause:** Package naming convention mismatch between `gen` and `impl` crates.

`brrtrouter client build` constructs the expected impl package name from the gen package name:

```
gen name:    api_keys_service_api
↓ brrtrouter appends "sesame_idam_..._impl"
expected:    sesame_idam_api_keys_gen_service_api_impl
actual:      sesame_idam_api_keys_gen_impl
→ MISMATCH
```

Hauliage's correct pattern:
```
gen name:    hauliage_company_gen
expected:    hauliage_company (just the prefix, no suffix)
actual:      hauliage_company
→ MATCH ✓
```

**Impact:** Cannot generate client SDKs, cannot use `brrtrouter client build` for any service.

### 3.2 MODERATE — Missing Build Infrastructure

| Missing Item | Expected Path | Hauliage Equivalent | Purpose |
|---|---|---|---|
| `build.rs` | `impl/build.rs` | `company/impl/build.rs` | Lifeguard migration generation from entity models |
| `config/service.yaml` | `impl/config/service.yaml` | `company/impl/config/service.yaml` | CORS, security providers, HTTP config |
| `services/` layer | `impl/src/services/` | `company/impl/src/services/` | Business logic abstraction (controllers call services) |
| `org_resolution.rs` | `impl/src/org_resolution.rs` | `identity/impl/src/org_resolution.rs` | Tenant/org ID resolution utility |
| `tests/` | `impl/tests/` | `company/impl/tests/` | BDD test suite (rstest-bdd) |
| `seeds/` | `impl/seeds/` | `company/impl/seeds/` | Test/seed data for development |

### 3.3 MINOR — Missing Patterns

| Missing Item | Purpose |
|---|---|
| `examples/` directory | Consumer example code |
| `may_postgres` `[patch]` in workspace Cargo.toml | Local development override |
| `sesame-audit` as workspace member | Currently at `microservices/sesame-audit` — verify it's in workspace members |

---

## 4. NAMING CONVENTION GAP

### 4.1 Current (Broken)

| Service | Gen Package Name | Impl Package Name |
|---|---|---|
| api-keys | `api_keys_service_api` | `sesame_idam_api_keys_gen_impl` |
| authz-core | `authz_core_service_api` | `sesame_idam_authz_core_gen_impl` |
| identity-login | `identity_login_service_service_api` | `sesame_idam_identity_login_service_gen_impl` |
| identity-session | `identity_session_service_service_api` | `sesame_idam_identity_session_service_gen_impl` |
| identity-user-mgmt | `identity_user_mgmt_service_service_api` | `sesame_idam_identity_user_mgmt_service_gen_impl` |
| org-mgmt | `org_mgmt_service_api` | `sesame_idam_org_mgmt_gen_impl` |

### 4.2 Target (Aligned with Hauliage)

| Service | Gen Package Name | Impl Package Name |
|---|---|---|
| api-keys | `sesame_idam_api_keys_gen` | `sesame_idam_api_keys` |
| authz-core | `sesame_idam_authz_core_gen` | `sesame_idam_authz_core` |
| identity-login | `sesame_idam_identity_login_service_gen` | `sesame_idam_identity_login_service` |
| identity-session | `sesame_idam_identity_session_service_gen` | `sesame_idam_identity_session_service` |
| identity-user-mgmt | `sesame_idam_identity_user_mgmt_service_gen` | `sesame_idam_identity_user_mgmt_service` |
| org-mgmt | `sesame_idam_org_mgmt_gen` | `sesame_idam_org_mgmt` |

### 4.3 Database Crate

Current: `database` (at `microservices/database/`)
Target: `sesame_idam_database` (aligned with prefix convention)

---

## 5. REMEDIATION PLAN

### Phase 1: Fix Package Naming (CRITICAL — unblocks everything)

**Scope:** All 12 `Cargo.toml` files (6 gen + 6 impl)

**Changes per gen crate:**
```toml
# BEFORE
[package]
name = "api_keys_service_api"

# AFTER
[package]
name = "sesame_idam_api_keys_gen"
```

**Changes per impl crate:**
```toml
# BEFORE
[package]
name = "sesame_idam_api_keys_gen_impl"

# AFTER
[package]
name = "sesame_idam_api_keys"
```

**Dependency updates needed in impl `Cargo.toml`:**
```toml
# BEFORE
[dependencies]
sesame_idam_api_keys_service_api = { path = "../gen" }

# AFTER
[dependencies]
sesame_idam_api_keys_gen = { path = "../gen" }
```

**Workspace `Cargo.toml` update:**
```toml
# BEFORE
"impl/api-keys/*",
"gen/api-keys/*",

# AFTER (keep wildcard, but rename files/dirs)
"impl/api-keys/*",
"gen/api-keys/*",
# (wildcard matches remain the same — only Cargo.toml contents change)
```

**Cross-reference files updated:**
- All 6 impl `Cargo.toml` dependency declarations pointing to gen packages
- All 6 impl `src/main.rs` imports referencing gen types
- All 6 gen `src/main.rs` binary name declarations

**Risk:** Low. Changes are mechanical (string replacement across known files). Workspace wildcard members mean no `Cargo.toml` members list change needed.

### Phase 2: Add Build Infrastructure (MODERATE — production readiness)

**2.1 Add `build.rs` to each impl crate**

Reference: `hauliage/company/impl/build.rs`
- Reads entity models from `src/models/`
- Generates migrations via lifeguard-migrate
- Creates `migrations/` output directory

**2.2 Add `config/service.yaml` to each impl crate**

Reference: `hauliage/company/impl/config/service.yaml`
- CORS configuration
- Security provider configuration
- HTTP server configuration (address, port, timeouts)
- Database pool configuration

**2.3 Add `services/` layer to each impl crate**

Reference: `hauliage/company/impl/src/services/`
- Controllers call services (not database directly)
- Services implement business logic
- Services receive `PooledLifeExecutor` for DB access

### Phase 3: Add Supporting Files (MINOR — completeness)

**3.1 Add `org_resolution.rs`** to each impl service
- Tenant/org ID resolution from request context
- Reference: `hauliage/identity/impl/src/org_resolution.rs`

**3.2 Add `tests/` directory with BDD test skeleton**
- Reference: `hauliage/company/impl/tests/`
- Minimal smoke test for at least one endpoint per service

**3.3 Add `seeds/` directory with development seed data**
- Reference: `hauliage/company/impl/seeds/`
- Seed data for local development/testing

### Phase 4: Workspace Cleanup

**4.1 Rename database crate** from `database` to `sesame_idam_database`
- Update `Cargo.toml` name field
- Update all dependency references across 6 impl crates
- Update workspace member path if needed

**4.2 Add `may_postgres` [patch] to workspace `Cargo.toml`**
- Reference: `hauliage/microservices/Cargo.toml` `[patch.crates-io]`

---

## 9. IMPLEMENTATION ORDER

### Phase 0: Tiltfile Rewrite (Parallel with Phase 1)
The Tiltfile is completely broken and needs a rewrite. This can happen in parallel with Phase 1 since the Tiltfile doesn't affect `cargo check`.

**Steps:**
0a. Create `bff-suite-config.yaml` with 6 services + ports (needed for Tiltfile service discovery)
0b. Create `docker/microservices/Dockerfile.template` (copy from hauliage, adapt paths)
0c. Create `docker/base/Dockerfile` + `dev-entrypoint.sh` (copy from hauliage)
0d. Create minimal Helm chart or k8s_yaml templates for deployments
0e. Rewrite Tiltfile from hauliage pattern, adapted for sesame-idam:
   - Fix cluster context: `kind-kind` (not `kind-sesame-idam`)
   - Fix port: `10351` (already set in justfile)
   - Add `SERVICE_PORTS` dict for all 6 services
   - Add `PACKAGE_NAMES` dict (post-Phase-1 names)
   - Implement full resource pipeline per service (lint → gen → build → docker → deploy)
   - Add live_update sync paths
   - Add data infrastructure wiring (Redis PV, database env)
0f. Validate with `tilt up --dry-run` or `tilt lint`

**Dependencies:** Phase 0 can start immediately. Phase 0 step 0e (package names) depends on Phase 1 completion.

### Phase 1: Fix Package Naming (CRITICAL — unblocks everything)
(unchanged from above)

### Phase 2: Add Build Infrastructure (MODERATE — production readiness)
(unchanged from above)

### Phase 3: Add Supporting Files (MINOR — completeness)
(unchanged from above)

### Phase 4: Workspace Cleanup
(unchanged from above)

### Phase 5: Tiltfile Validation & Data Wiring
5a. Run `tilt up` and verify all 6 services start
5b. Verify port forwards work (postgres 5432, redis 6379)
5c. Verify live_update sync (edit source → save → service restarts)
5d. Create database env manifests (secrets/configmaps for postgres)
5e. Wire Redis deployment via K8s manifest

---

## 6. TILT & TOOLING ARCHITECTURE (NEW SECTION)

### 6.1 Current Tooling Stack

```
seasame-idam/
├── tooling/                          ← Sesame-specific tooling package
│   ├── pyproject.toml                ← Package: sesame-idam-tooling [dev]
│   └── src/sesame_idam_tooling/
│       ├── cli/main.py               ← Thin shim: delegates to brrtrouter_tooling
│       └── tilt/
│           ├── setup_kind_registry.py
│           └── setup_persistent_volumes.py
├── justfile                          ← Full justfile (init, gen, lint, serve, dev-up, tilt-*)
└── Tiltfile                          ← BROKEN — auto-generated with template failures
```

**The `sesame-idam` CLI bin** (`~/.local/share/brrtrouter/venv/bin/sesame-idam`) is a thin shim:
```python
from brrtrouter_tooling.workspace.cli.main import main
```
It exposes all hauliage/BRRTRouter tooling commands (ports, openapi, ci, bff, docker, gen, build, bootstrap, release, tilt, pre-commit) through the same interface.

### 6.2 Tiltfile Status: BROKEN (Auto-generated with Template Failures)

The existing Tiltfile has **multiple critical issues**:

**Template variable interpolation failures:**
- Uses `%%` in Starlark string context (should be `%` or `.format()`): `'%s' %% brrtrouter_root`
- Uses Python f-string syntax inside Starlark: `$_s` variable substitution in `str(local('... $_s ...'))`
- String formatting mixed with Python `%` operator: `'sesame-idam_bin lint \"%s\" % (name)'` — but `sesame-idam_bin` is a path string, not a format specifier
- Broken `get_package_name()` function: references undefined `fallback` variable, uses `%%` for format
- Broken `create_microservice_lint()`: `deps=` is outside the `local_resource()` call parameters

**Missing infrastructure references:**
- `bff-suite-config.yaml` does not exist (but Tiltfile tries to parse it at line 134)
- `helm/sesame-idam-microservice/` directory does not exist (line 284)
- `docker/microservices/Dockerfile.template` does not exist (line 278)
- `k8s/microservices/` is referenced but only contains `namespace.yaml`

**Cluster configuration issues:**
- Uses `kind-sesame-idam` context instead of shared `kind-kind` (line 4)
- Tilt UI on port 10351 (conflicts with dev-up default of 10351 in justfile — OK, consistent)

### 6.3 Hauliage Tiltfile Reference (Working Pattern)

Hauliage's Tiltfile defines the complete development pipeline. Here's the flow:

```
Startup Order (per service):
─────────────────────────────
1. build-tooling          → pip install -e BRRTRouter/tooling[dev] + sesame-idam tooling[dev]
2. build-base-image       → docker build hauliage-base (manages base image with predictable :latest tag)
3. [per service] yaml-validate  → js-yaml duplicate-key detection (runs BEFORE cargo lint)
4. [per service] lint             → brrtrouter-gen lint --spec --fail-on-error
5. [per service] gen                → hauliage gen suite hauliage --service <name>
                                       (uses brrtrouter-gen under the hood, also runs fix_cargo_paths)
6. [per service] build            → hauliage build microservice <name>
                                       (cargo zigbuild for cross-compile to Linux x86_64 musl)
7. [per service] copy             → hauliage docker copy-binary (binary → build_artifacts/)
8. [per service] docker           → hauliage docker build-image-simple (uses template)
9. [per service] custom_build     → Tilt live_update with binary + config + doc + static_site syncs
10. [per service] k8s_yaml       → Helm deployment (namespace: hauliage)
11. [per service] k8s_resource   → Port forward, resource deps, labels

Data Infrastructure:
────────────────────
- k8s_yaml(kustomize('k8s/data/supabase'))    ← bundled stack only
- k8s_yaml('k8s/microservices/database-env.yaml')  ← database config + secrets
- Redis: separate k8s/data/redis.yaml or shared-kind-cluster
- Port forwards: postgres 5432, redis 6379
```

**Key Hauliage Tiltfile patterns we must replicate:**
- `docker_prune_settings` to prevent disk exhaustion
- `local_resource` with `ignore` patterns to avoid build storms
- `deps` and `resource_deps` for correct build ordering
- `custom_build` with `live_update` (sync + kill -HUP for hot reload)
- `k8s_yaml` with `kustomize` or `helm` for deployments
- `k8s_resource` with `port_forwards`, `labels`, `resource_deps`
- Port registry via `PACKAGE_NAMES` dict
- Shared Kind infra vs bundled data stack toggle

### 6.4 Justfile (Working — Better Than Tiltfile)

Sesame's justfile is functional and defines:

**Codegen per service** (`just gen-<service>`):
```bash
cd BRRTRouter && cargo run --bin brrtrouter-gen -- generate \
  --spec $(pwd)/openapi/idam/<service>/openapi.yaml \
  --output $(pwd)/microservices/idam/<service>/gen \
  --package-name <service>_service_api \
  --force
```

**Lint per service** (`just lint-openapi-<service>`):
```bash
cd BRRTRouter && cargo run --bin brrtrouter-gen -- lint \
  --spec $(pwd)/openapi/idam/<service>/openapi.yaml \
  --fail-on-error
```

**Serve per service** (`just serve-<service>`):
```bash
cd BRRTRouter && cargo run --bin brrtrouter-gen -- serve \
  --spec $(pwd)/openapi/idam/<service>/openapi.yaml \
  --addr <addr>
```

**Dev environment** (`just dev-up`):
1. Verify shared Kind cluster exists (`kind-kind` context)
2. `sesame tilt setup-kind-registry` — set up localhost:5001 Docker registry
3. `kubectl apply -f k8s/microservices/namespace.yaml`
4. `sesame tilt setup-persistent-volumes` — Redis PVs
5. `mkdir -p /tmp/sesame-idam-data/` — host dirs for PVs
6. `tilt up --host=0.0.0.0 --port=10351`

**Supabase** (`just supabase-apply`):
- `kubectl apply -k microscaler-supabase/k8s/overlays/seasame-idam`

**Port forwarding** (`just port-forward`):
- postgres: `kubectl port-forward -n data svc/postgres 5432:5432`
- redis: `kubectl port-forward -n sesame-idam svc/redis 6379:6379`

**Tilt systemd service** (`just tilt-up/tilt-down/tilt-log`):
- Managed via `systemctl --user start tilt-sesame-idam.service`
- Port 10351

### 6.5 BRRTRouter Tooling Commands (Under the Hood)

All sesame-idam CLI commands delegate to `brrtrouter_tooling.workspace`:

| Command | Implementation | Purpose |
|---------|---------------|---------|
| `sesame gen suite idam` | `brrtrouter_tooling.gen.regenerate.gen_suite` | Generate gen crates from OpenAPI specs, run fix_cargo_paths |
| `sesame gen stubs` | `brrtrouter_tooling.gen.regenerate.gen_stubs` | Regenerate impl controller stubs via brrtrouter-gen |
| `sesame build microservice` | `brrtrouter_tooling.build.workspace_build.build_microservice` | cargo zigbuild for cross-compile to Linux musl |
| `sesame docker copy-binary` | `brrtrouter_tooling.docker.copy_binary` | Copy binary to build_artifacts/ staging dir |
| `sesame docker build-image-simple` | `brrtrouter_tooling.docker.build_image_simple` | Render Dockerfile template, build image |
| `sesame docker build-base` | `brrtrouter_tooling.docker.build_base` | Build base image with dev-entrypoint.sh |
| `sesame openapi lint` | `brrtrouter_tooling.openapi.validate.validate` | Validate OpenAPI spec |
| `sesame ports list` | `brrtrouter_tooling.workspace.ports.list_ports` | List assigned ports |
| `sesame tilt setup-kind-registry` | `sesame_idam_tooling.tilt.setup_kind_registry` | Set up localhost:5001 Docker registry in Kind |
| `sesame tilt setup-persistent-volumes` | `sesame_idam_tooling.tilt.setup_persistent_volumes` | Create PVs for Redis, Postgres, etc. |

### 6.6 Docker `build-image-simple` CLI Argument Mismatch — Root Cause of Tilt Failure

**This is the #1 blocker preventing sesame-idam from building Docker images in Tilt.**

The underlying brrtrouter_tooling `build-image-simple` command expects:

```
sesame-idam docker build-image-simple <image_name> <dockerfile> <hash_path> <artifact_path> [--service <name>]
```

When `--service <name>` is provided, the `dockerfile` argument is treated as a `Dockerfile.template` path and rendered on the fly. Without it, `dockerfile` must be an actual Dockerfile.

The sesame-idam Tiltfile (`Tiltfile` line ~256) was calling it with WRONG arguments and WRONG flags:

| Aspect | Hauliage (Correct) | Sesame-IDAM Current (Broken) | Sesame-IDAM Future (Corrected) |
|---|---|---|---|
| **Full CLI signature** | `build-image-simple <image> <dockerfile_template> <hash_path> <artifact_path> --service <name>` | `build-image-simple <image> <hash_path> <artifact_path> --system S --module M --port N --binary-name B` | `build-image-simple <image> <dockerfile_template> <hash_path> <artifact_path> --service <name>` |
| **dockerfile arg** | `docker/microservices/Dockerfile.template` (template path) | **MISSING** — arg shifted to `<hash_path>` position | `docker/microservices/Dockerfile.template` |
| **hash_path arg** | `build_artifacts/<arch>/<service>.sha256` | `build_artifacts/<service>.sha256` (no arch subdir) | `build_artifacts/<arch>/<service>.sha256` |
| **artifact_path arg** | `build_artifacts/<arch>/<service>` (full path with arch dir) | `build_artifacts/<service>` (no arch dir) | `build_artifacts/<arch>/<service>` |
| **Service identification** | `--service <name>` (single flag) | `--system idam --module <name> --port <port> --binary-name <pkg>` (4 flags) | `--service <name>` |
| **binary_name derivation** | Uses service slug with dashes → `_` conversion: `name.replace('-', '_')` | Uses Cargo package name directly: `binary_name = package_name` | Uses service slug with dashes → `_` conversion |
| **artifact_path derivation** | `'build_artifacts/%s/%s' % (TARGET_ARCH_NAME, binary_name)` | `'build_artifacts/%s' % binary_name` (no arch) | `'build_artifacts/%s/%s' % (TARGET_ARCH_NAME, binary_name)` |
| **Architecture detection** | Dynamic: `uname -m` → `TARGET_ARCH_NAME` + `TARGET_RUST_TRIPLE` | **MISSING** — hardcoded `x86_64-unknown-linux-musl` | Dynamic: `uname -m` → `TARGET_ARCH_NAME` + `TARGET_RUST_TRIPLE` |
| **resource_deps** | `['build-base-image', 'copy-%s' % name]` | `['copy-%s' % name]` (missing base image) | `['build-base-image', 'copy-%s' % name]` |
| **deps array** | `[hash_path, artifact_path, dockerfile_template, 'tooling/pyproject.toml']` | `[artifact_path, '%s/%s.sha256' % ('build_artifacts', binary_name), dockerfile_template, 'docker/base/Dockerfile', 'tooling/pyproject.toml']` | `[hash_path, artifact_path, dockerfile_template, 'tooling/pyproject.toml']` |
| **allow_parallel** | `False` (sequential to avoid docker contention) | `True` | `False` |
| **custom_build with live_update** | Yes — syncs binary, config, doc, static_site with `kill -HUP 1` reload | **MISSING** — no hot-reload support | Yes — syncs binary, config, doc, static_site with `kill -HUP 1` reload |
| **Example command** | `sesame-idam docker build-image-simple localhost:5001/sesame-idam-org-mgmt docker/microservices/Dockerfile.template build_artifacts/amd64/org_mgmt.sha256 build_artifacts/amd64/org_mgmt --service org-mgmt` | `sesame-idam docker build-image-simple localhost:5001/sesame-idam-org-mgmt build_artifacts/org_mgmt.sha256 build_artifacts/org_mgmt --system idam --module org-mgmt --port 8104 --binary-name sesame_idam_org_mgmt` | `sesame-idam docker build-image-simple localhost:5001/sesame-idam-org-mgmt docker/microservices/Dockerfile.template build_artifacts/amd64/org_mgmt.sha256 build_artifacts/amd64/org_mgmt --service org-mgmt` |

**Why the current sesame-idam call fails:**

1. The CLI receives `build_artifacts/org_mgmt.sha256` as the `dockerfile` argument (not `docker/microservices/Dockerfile.template`).
2. The actual template path `docker/microservices/Dockerfile.template` is never passed to the CLI — it's referenced only in the Tiltfile `deps=` array.
3. Without `--service <name>`, the CLI does NOT render the template; it tries to use the first arg as a real Dockerfile.
4. The `--system`/`--module`/`--port`/`--binary-name` flags are not recognized by `build-image-simple` — they are remnants from other commands like `generate-dockerfile`.
5. The `artifact_path` lacks the `<arch>/` subdirectory, so even if the binary were found, the path wouldn't resolve.

**Actual error observed:** `❌ Artifact not found: /home/casibbald/Workspace/microscaler/seasame-idam/sesame_idam_org_mgmt`

The CLI looks for the artifact at the wrong path because `artifact_path` is `build_artifacts/org_mgmt` instead of `build_artifacts/amd64/org_mgmt`, and the dockerfile_template was never rendered.

### 6.7 Tiltfile Rewrite Plan

The Tiltfile needs a complete rewrite following the hauliage pattern but adapted for sesame-idam:

```python
# Required variables (already defined in broken Tiltfile):
- brrtrouter_root = '../BRRTRouter'
- brrtrouter_venv = '~/.local/share/brrtrouter/venv'
- sesame_bin = '%s/bin/sesame-idam' % brrtrouter_venv
- brrtrouter_gen_bin = '%s/target/debug/brrtrouter-gen' % brrtrouter_root

# Cluster config:
- allow_k8s_contexts(['kind-kind'])  # NOT kind-sesame-idam
- namespace: 'sesame-idam'
- port: 10351

# Dynamic architecture selection (must be at module level, outside functions):
host_machine = str(local('uname -m', quiet=True)).strip()
if host_machine in ['arm64', 'aarch64']:
    TARGET_ARCH_NAME = 'arm64'
    TARGET_RUST_TRIPLE = 'aarch64-unknown-linux-musl'
else:
    TARGET_ARCH_NAME = 'amd64'
    TARGET_RUST_TRIPLE = 'x86_64-unknown-linux-musl'

# Service port mapping (from justfile/ports config):
SERVICE_PORTS = {
    'identity-login-service': '8101',
    'identity-session-service': '8105',
    'identity-user-mgmt-service': '8106',
    'authz-core': '8102',
    'api-keys': '8103',
    'org-mgmt': '8104',
}

# Package name mapping (after Phase 1 naming fix):
PACKAGE_NAMES = {
    'identity-login-service': 'sesame_idam_identity_login_service',
    'identity-session-service': 'sesame_idam_identity_session_service',
    'identity-user-mgmt-service': 'sesame_idam_identity_user_mgmt_service',
    'authz-core': 'sesame_idam_authz_core',
    'api-keys': 'sesame_idam_api_keys',
    'org-mgmt': 'sesame_idam_org_mgmt',
}

# Per-service deployment template (MUST match hauliage pattern):
def create_microservice_deployment(name, port):
    package_name = PACKAGE_NAMES.get(name, 'sesame_idam_' + name.replace('-', '_'))
    binary_name = name.replace('-', '_')  # service slug → binary name
    target_path = 'microservices/target/%s/debug/%s' % (TARGET_RUST_TRIPLE, package_name)
    artifact_path = 'build_artifacts/%s/%s' % (TARGET_ARCH_NAME, binary_name)
    hash_path = 'build_artifacts/%s/%s.sha256' % (TARGET_ARCH_NAME, binary_name)
    dockerfile_template = 'docker/microservices/Dockerfile.template'
    image_name = 'localhost:5001/sesame-idam-%s' % name

    # 1. Copy binary from workspace build to artifacts and create SHA256 hash
    local_resource('copy-%s' % name,
        '%s docker copy-binary %s %s %s' % (sesame_idam_bin, target_path, artifact_path, binary_name),
        deps=[target_path, 'tooling/pyproject.toml'],
        resource_deps=['build-%s' % name],
        labels=[name], allow_parallel=True)

    # 2. Build and push Docker image (template rendered on the fly with --service)
    local_resource('docker-%s' % name,
        '%s docker build-image-simple %s %s %s %s --service %s' % (
            sesame_idam_bin, image_name, dockerfile_template, hash_path, artifact_path, name),
        deps=[hash_path, artifact_path, dockerfile_template, 'tooling/pyproject.toml'],
        resource_deps=['build-base-image', 'copy-%s' % name],
        labels=[name], allow_parallel=False)

    # 3. Custom build for Tilt live updates (binary + config + doc + static_site syncs)
    custom_build(image_name,
        ('%s docker build-image-simple %s %s %s %s --service %s' % (
            sesame_idam_bin, image_name, dockerfile_template, hash_path, artifact_path, name)
         + ' && (docker push %s:tilt 2>/dev/null || kind load docker-image %s:tilt --name sesame-idam)' % (image_name, image_name)),
        deps=[artifact_path, hash_path, dockerfile_template,
              'microservices/idam/%s/impl/config' % name,
              'microservices/idam/%s/gen/doc' % name,
              'microservices/idam/%s/gen/static_site' % name],
        tag='tilt',
        live_update=[
            sync(artifact_path, '/app/%s' % binary_name),
            sync('microservices/idam/%s/impl/config/' % name, '/app/config/'),
            sync('microservices/idam/%s/gen/doc/' % name, '/app/doc/'),
            sync('microservices/idam/%s/gen/static_site/' % name, '/app/static_site/'),
            run('kill -HUP 1', trigger=[artifact_path]),
        ])

    # 4. Deploy using Helm
    _helm_values = ['helm/sesame-idam-microservice/values/%s.yaml' % name]
    k8s_yaml(helm('helm/sesame-idam-microservice', name=name, namespace='sesame-idam', values=_helm_values))

    # 5. Kubernetes resource configuration
    k8s_resource(name,
        port_forwards=['%s:%s' % (port, port)],
        resource_deps=['docker-%s' % name],
        labels=[name], auto_init=True, trigger_mode=TRIGGER_MODE_AUTO)
```

### 6.8 Data Infrastructure

| Component | Namespace | Source | Tiltfile Action |
|---|---|---|---|
| **PostgreSQL** | `data` | Shared Kind (microscaler-supabase) | `k8s_yaml(kustomize('k8s/data/supabase'))` — via Supabase overlay |
| **Redis** | `sesame-idam` | `k8s/data/redis.yaml` | Apply via kustomize or k8s_yaml |
| **Persistent Volumes** | `sesame-idam` | `k8s/data/persistent-volumes.yaml` | Via `sesame tilt setup-persistent-volumes` |
| **Local Registry** | N/A | Docker registry in Kind | `sesame tilt setup-kind-registry` |
| **Database secrets** | `sesame-idam` | `k8s/microservices/database-env.yaml` (MISSING) | Need to create (postgres password, connection strings) |

---

## 7. RISK ASSESSMENT

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Breaking existing stub controllers | Low | Low | Phase 1 is purely mechanical string replacement |
| Breaking database crate references | Low | Medium | All references are in Cargo.toml — grep finds them all |
| Misnaming gen dependency paths | Low | Medium | Use find/grep to audit all path = "../gen" references |
| Build.rs conflicts with existing models | Low | Low | Follow hauliage pattern exactly |
| Config.yaml format incompatibility | Low | Low | Follow hauliage format exactly |
| **Tiltfile rewrite breaks Tilt startup** | Medium | High | Validate Tiltfile with `tilt lint` or dry-run `tilt up --dry-run` |
| **Missing Dockerfile template breaks image builds** | High | High | Create template from hauliage's pattern, adapt paths |
| **Helm charts missing breaks K8s deployments** | High | High | Create minimal Helm chart or use k8s_yaml with templates |

---

## 8. ACCEPTANCE CRITERIA

- [ ] `cargo check --workspace` passes with 0 errors
- [ ] `cargo test --workspace` passes (existing 4 + 1 doc test)
- [ ] `brrtrouter client build` succeeds for all 6 services
- [ ] No `non_snake_case` warnings introduced (existing ones are from gen code)
- [ ] No dead_code warnings for new code
- [ ] All gen→impl dependency paths resolve correctly
- [ ] Package naming follows `sesame_idam_<service>_gen` / `sesame_idam_<service>` convention
- [ ] `sesame_idam_database` crate properly renamed and referenced

---

## 9. OUT OF SCOPE

- Implementing actual business logic in stub controllers (separate PRD)
- OpenAPI spec changes (specs are correct, only package naming is wrong)
- Hauliage changes (this only affects sesame-idam)
- Authentication/authorization implementation
- Database migration content (only build.rs for generation, not the SQL)

---

## 10. OPEN QUESTIONS

1. **Should `seasame_idam_database` be moved to workspace root (`microservices/`) or nested under `microservices/idam/`?**
   - Current: `microservices/database/` at workspace root
   - Hauliage: `hauliage_database/` at workspace root
   - **Decision:** Move to `microservices/idam/seasame_idam_database/` to align with the suite-based layout. The `idam/` directory becomes the suite root, containing all IDAM-specific crates (6 gen + 6 impl + the shared database crate). This groups the database layer with the services that consume it, matching the suite boundary.
   - Changes needed: rename `microservices/database/` → `microservices/idam/seasame_idam_database/`, update `[package].name` to `seasame_idam_database`, update all 6 impl `Cargo.toml` dep paths from `../database` to `../seasame_idam_database`, update workspace `Cargo.toml` members.

2. **Does `seasame-audit` need to be moved into the `idam/` subdirectory for consistency?**
   - Current: `microservices/sesame-audit/` (at workspace root)
   - Hauliage's equivalent (`email_reminder_worker`) is at `microservices/` root
   - **Decision:** Move to `microservices/idam/seasame_audit/` to consolidate all IDAM suite crates under one directory. The `idam/` suite root contains everything: `seasame_idam_database/`, `seasame_audit/`, 6 `gen/` crates, 6 `impl/` crates. This makes the suite boundary explicit and simplifies workspace member paths.
   - Changes needed: rename `microservices/sesame-audit/` → `microservices/idam/seasame_audit/`, update workspace `Cargo.toml` members.

3. **Workspace structure — consolidate everything under `microservices/idam/`?**
   - Hauliage puts services directly under `microservices/` (no suite prefix dir)
   - Sesame already uses `microservices/idam/{service}/` pattern
   - **Decision:** Keep `idam/` as the suite root. Move `database/` → `microservices/idam/seasame_idam_database/` and `seasame-audit/` → `microservices/idam/seasame_audit/`. Final layout:
   ```
   microservices/
   ├── Cargo.toml                 ← workspace, members include "idam/*"
   └── idam/                      ← IDAM suite root
       ├── api-keys/
       │   ├── gen/               ← sesame_idam_api_keys_gen
       │   └── impl/              ← sesame_idam_api_keys
       ├── authz-core/
       │   ├── gen/               ← sesame_idam_authz_core_gen
       │   └── impl/              ← sesame_idam_authz_core
       ├── identity-login-service/
       │   ├── gen/               ← sesame_idam_identity_login_service_gen
       │   └── impl/              ← sesame_idam_identity_login_service
       ├── identity-session-service/
       │   ├── gen/               ← sesame_idam_identity_session_service_gen
       │   └── impl/              ← sesame_idam_identity_session_service
       ├── identity-user-mgmt-service/
       │   ├── gen/               ← sesame_idam_identity_user_mgmt_service_gen
       │   └── impl/              ← sesame_idam_identity_user_mgmt_service
       ├── org-mgmt/
       │   ├── gen/               ← sesame_idam_org_mgmt_gen
       │   └── impl/              ← sesame_idam_org_mgmt
       ├── seasame_idam_database/ ← shared PooledLifeExecutor crate
       └── seasame_audit/         ← HMAC-signed audit event crate
   ```
   - Changes needed: rename two crate dirs, update workspace `Cargo.toml` members to `"idam/*"`, update all inter-crate dep paths, update Tiltfile `create_microservice_gen()` and other functions that reference `microservices/idam/` paths (already correct for services, just needs database/audit path adjustments).
