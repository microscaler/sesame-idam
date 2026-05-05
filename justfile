# Sesame-IDAM justfile
# Repo layout: 4 services split by access pattern:
#   identity-auth  — user-facing identity/authentication (8001)
#   authz-core     — per-request authorization checks (8002)
#   api-keys       — M2M key management/validation (8003)
#   org-mgmt       — org lifecycle & SSO admin (8004)
# OpenAPI specs: openapi/{identity-auth,authz-core,api-keys,org-mgmt}/openapi.yaml
# Consumes shared BRRTRouter tooling (brrtrouter-gen) for codegen, lint, serve.
# Set BRRTRouter_DIR if BRRTRouter is not a sibling repo (e.g. export BRRTRouter_DIR=/path/to/BRRTRouter).

set shell := ["bash", "-uc"]

# BRRTRouter repo path (sibling of seasame-idam)
# Override with: BRRTRouter_DIR=/path/to/BRRTRouter just lint-openapi
brrtrouter_dir := "../BRRTRouter"

# microscaler-supabase side-clone (for Supabase stack)
# Override with: SUPABASE_DIR=/path/to/microscaler-supabase just supabase-apply
supabase_dir := "../microscaler-supabase"

# OpenAPI spec paths (4 services split by access frequency & cost)
spec_identity_auth  := "openapi/identity-auth/openapi.yaml"
spec_authz_core     := "openapi/authz-core/openapi.yaml"
spec_api_keys       := "openapi/api-keys/openapi.yaml"
spec_org_mgmt       := "openapi/org-mgmt/openapi.yaml"

# Output dirs for brrtrouter-gen (gen crates live under each microservice)
out_identity_auth  := "microservices/idam/identity-auth/gen"
out_authz_core     := "microservices/idam/authz-core/gen"
out_api_keys       := "microservices/idam/api-keys/gen"
out_org_mgmt       := "microservices/idam/org-mgmt/gen"

default:
  @just --list --unsorted

# =============================================================================
# Tooling (.venv and sesame CLI) — same guard rails as RERP
# =============================================================================
# Run `just init` once before first use of lint/format/qa/install-hooks.

# Create tooling/.venv and install sesame-idam-tooling [dev]. Idempotent.
init:
  #!/usr/bin/env bash
  set -euo pipefail
  echo "🐍 Setting up tooling .venv..."
  if [ ! -d tooling/.venv ]; then
    python3 -m venv tooling/.venv
  fi
  tooling/.venv/bin/pip install --upgrade pip
  tooling/.venv/bin/pip install -e ./tooling[dev]
  echo "✅ Tooling .venv ready. Use: tooling/.venv/bin/sesame or add tooling/.venv/bin to PATH"

# Rebuild tooling (pip install -e) after source changes. Run `just init` first.
build-tooling:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d tooling/.venv ]; then
    echo "❌ tooling/.venv not found. Run: just init"
    exit 1
  fi
  echo "🔨 Rebuilding tooling..."
  tooling/.venv/bin/pip install -e ./tooling[dev]
  echo "✅ Tooling rebuilt"

# Start an interactive shell with tooling/.venv activated (exit to leave).
venv:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d tooling/.venv ]; then
    echo "❌ tooling/.venv not found. Run: just init"
    exit 1
  fi
  echo "Shell with tooling/.venv activated (exit to leave)..."
  exec bash -c "source tooling/.venv/bin/activate && exec bash -i"

# Run ruff check on tooling (same rules as RERP). Run `just init` first.
lint:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d tooling/.venv ]; then
    echo "❌ tooling/.venv not found. Run: just init"
    exit 1
  fi
  tooling/.venv/bin/ruff check tooling/

# Format tooling with ruff. Run `just init` first.
format:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d tooling/.venv ]; then
    echo "❌ tooling/.venv not found. Run: just init"
    exit 1
  fi
  tooling/.venv/bin/ruff format tooling/

# Check tooling is formatted (CI). Run `just init` first.
format-check:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d tooling/.venv ]; then
    echo "❌ tooling/.venv not found. Run: just init"
    exit 1
  fi
  tooling/.venv/bin/ruff format tooling/ --check

# Full QA: lint + format-check + tooling tests. Run before commit or demo.
qa:
  #!/usr/bin/env bash
  set -euo pipefail
  just lint
  just format-check
  tooling/.venv/bin/pytest tooling/tests -v --tb=short

# Auto-fix fixable ruff rules (including unsafe). Run `just init` first.
lint-fix:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d tooling/.venv ]; then
    echo "❌ tooling/.venv not found. Run: just init"
    exit 1
  fi
  tooling/.venv/bin/ruff check tooling/ --fix --unsafe-fixes

# Install pre-commit hooks (qa, check-empty-print). Run `just init` first.
install-hooks:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d tooling/.venv ]; then
    echo "❌ tooling/.venv not found. Run: just init"
    exit 1
  fi
  tooling/.venv/bin/pre-commit install
  echo "✅ Pre-commit hooks installed"

# Find and remove unused imports in tooling (F401). Run `just init` first.
lint-unused-imports:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d tooling/.venv ]; then
    echo "❌ tooling/.venv not found. Run: just init"
    exit 1
  fi
  echo "🔍 Checking for unused imports (F401) in tooling..."
  tooling/.venv/bin/ruff check tooling/ --select F401 --fix

# =============================================================================
# Development Environment (Kind + Tilt) — same DX as RERP
# =============================================================================
# Run `just init` before first dev-up so sesame tilt setup-kind-registry is available.

# Start development environment (shared Kind cluster; owned by shared-kind-cluster).
# Platform infra (postgres, postgres-meta, parquet-lake) lives in namespace data
# from shared-kind-cluster. Sesame-IDAM adds Redis in namespace sesame-idam.
dev-up:
  #!/usr/bin/env bash
  set -euo pipefail
  echo "🚀 Starting Sesame-IDAM development environment..."
  if [ ! -d tooling/.venv ]; then
    echo "❌ tooling/.venv not found. Run: just init"
    exit 1
  fi

  # Verify shared Kind cluster exists (owned by shared-kind-cluster; DO NOT create/delete here)
  echo "📦 Checking shared Kind cluster..."
  if ! kind get clusters 2>/dev/null | grep -qxF kind; then
    echo "[FAIL] Shared Kind cluster not found."
    echo "  Create it: cd ../shared-kind-cluster && just dev-up"
    exit 1
  fi
  echo "[OK] Shared Kind cluster exists"

  echo "📦 Setting up local registry (localhost:5001)..."
  tooling/.venv/bin/sesame tilt setup-kind-registry
  echo "⏳ Waiting for cluster to be ready..."
  kubectl wait --for=condition=Ready nodes --all --timeout=300s

  # Create sesame-idam namespace (Tilt does not manage it)
  echo "📁 Creating sesame-idam namespace..."
  kubectl apply -f k8s/microservices/namespace.yaml

  echo "💾 Creating PersistentVolumes (Redis PV)..."
  tooling/.venv/bin/sesame tilt setup-persistent-volumes || true

  echo "📦 Creating data dir on host for PVs (if using extraMounts)..."
  mkdir -p /tmp/sesame-idam-data/postgres /tmp/sesame-idam-data/parquet-lake /tmp/sesame-idam-data/redis /tmp/sesame-idam-data/prometheus /tmp/sesame-idam-data/grafana

  echo "📦 Apply Supabase stack once: just supabase-apply (then start Tilt)"
  echo "🎯 Starting Tilt (loads Redis, tooling; Postgres from microscaler-supabase in namespace data)..."
  tilt up --host=0.0.0.0 --port=10351

# Stop Sesame-IDAM Tilt only (cluster owned by shared-kind-cluster; registry left running)
dev-down:
  #!/usr/bin/env bash
  set -euo pipefail
  echo "🛑 Stopping Sesame-IDAM development environment..."
  pkill -f "tilt up" 2>/dev/null || true
  echo "✅ Development environment stopped"
  echo "   (Kind cluster unchanged — owned by shared-kind-cluster. Registry kind-registry left running.)"

# Stop development environment and remove the local registry
dev-down-full: dev-down
  @echo "🗑️ Removing local registry..."
  @docker stop kind-registry 2>/dev/null || true
  @docker rm kind-registry 2>/dev/null || true
  @echo "✅ Registry removed"

# Setup development environment (Tilt-based; cluster and namespace must exist)
setup:
  @tooling/.venv/bin/sesame tilt setup-kind-registry
  @tooling/.venv/bin/sesame tilt setup-persistent-volumes || true

# Teardown (add --remove-images, --remove-volumes as needed when sesame tilt teardown exists)
teardown:
  @echo "Run: just dev-down (and optionally just dev-down-full)"

# Start services with Tilt (cluster and namespace must exist; see dev-up)
up:
  @echo "Starting all services with Tilt..."
  @tilt up --host=0.0.0.0 --port=10351

# Start with Kind (cluster and namespace must exist)
up-k8s:
  @kubectl apply -f k8s/microservices/namespace.yaml 2>/dev/null || true
  @echo "Starting all services with Tilt (Kubernetes mode)..."
  @tilt up --host=0.0.0.0 --port=10351 -- --use-kind

# Stop Tilt
down:
  @tilt down --port 10351

# Show cluster and service status (shared cluster)
status:
  #!/usr/bin/env bash
  set -euo pipefail
  echo "Cluster status:"
  echo "  Current context: $(kubectl config current-context 2>/dev/null)"
  echo "  Kind clusters: $(kind get clusters 2>/dev/null | tr '\n' ', ' || echo 'none')"
  echo ""
  echo "Pods (sesame-idam):"
  kubectl get pods -n sesame-idam 2>/dev/null || echo "Namespace not found (run dev-up first)"
  echo ""
  echo "Services:"
  kubectl get svc -n sesame-idam 2>/dev/null || echo "No services found"

# Apply Supabase stack from microscaler-supabase (side-clone). Run before tilt up.
# Creates namespace data, postgres, postgres-meta, etc. Requires: just dev-up (cluster) first.
supabase-apply:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{supabase_dir}}" ]; then
    echo "❌ microscaler-supabase not found at {{supabase_dir}}. Clone as sibling or set SUPABASE_DIR."
    exit 1
  fi
  echo "📦 Applying Supabase stack (overlay seasame-idam)..."
  cd "{{supabase_dir}}" && kubectl apply -k k8s/overlays/seasame-idam
  echo "✅ Supabase stack applied (namespace: data). Run tilt up then just port-forward for postgres + redis."

# Port-forward PostgreSQL (namespace data) and Redis (namespace sesame-idam). Run after tilt up.
port-forward:
  #!/usr/bin/env bash
  set -euo pipefail
  echo "Setting up port forwards..."
  kubectl port-forward -n data svc/postgres 5432:5432 &
  kubectl port-forward -n sesame-idam svc/redis 6379:6379 &
  echo "Port forwards: postgres 5432 (data), redis 6379 (sesame-idam). Press Ctrl+C to stop."
  wait

# =============================================================================
# Workspace (Cargo)
# =============================================================================

# Check workspace (no members yet; validates Cargo.toml)
check:
  cargo check --workspace

# When microservices/gen are added: cargo build --release --workspace
# build:
#   cargo build --release --workspace

# =============================================================================
# BRRTRouter codegen (shared tooling)
# =============================================================================
# BRRTRouter codegen (shared tooling)
# =============================================================================

# Regenerate all 4 services from OpenAPI
gen: gen-identity-auth gen-authz-core gen-api-keys gen-org-mgmt

# Regenerate identity-auth (user-facing identity/authentication) gen crate
gen-identity-auth:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR or clone BRRTRouter as a sibling."
    exit 1
  fi
  echo "🔨 Generating identity-auth from {{spec_identity_auth}}..."
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- generate \
    --spec "$(cd - >/dev/null && pwd)/{{spec_identity_auth}}" \
    --output "$(cd - >/dev/null && pwd)/{{out_identity_auth}}" \
    --package-name sesame_idam_identity_auth_gen \
    --force
  echo "✅ Generated {{out_identity_auth}}"

# Regenerate authz-core (per-request authorization checks) gen crate
gen-authz-core:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR or clone BRRTRouter as a sibling."
    exit 1
  fi
  echo "🔨 Generating authz-core from {{spec_authz_core}}..."
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- generate \
    --spec "$(cd - >/dev/null && pwd)/{{spec_authz_core}}" \
    --output "$(cd - >/dev/null && pwd)/{{out_authz_core}}" \
    --package-name sesame_idam_authz_core_gen \
    --force
  echo "✅ Generated {{out_authz_core}}"

# Regenerate api-keys (M2M key management/validation) gen crate
gen-api-keys:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR or clone BRRTRouter as a sibling."
    exit 1
  fi
  echo "🔨 Generating api-keys from {{spec_api_keys}}..."
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- generate \
    --spec "$(cd - >/dev/null && pwd)/{{spec_api_keys}}" \
    --output "$(cd - >/dev/null && pwd)/{{out_api_keys}}" \
    --package-name sesame_idam_api_keys_gen \
    --force
  echo "✅ Generated {{out_api_keys}}"

# Regenerate org-mgmt (org lifecycle & SSO admin) gen crate
gen-org-mgmt:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR or clone BRRTRouter as a sibling."
    exit 1
  fi
  echo "🔨 Generating org-mgmt from {{spec_org_mgmt}}..."
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- generate \
    --spec "$(cd - >/dev/null && pwd)/{{spec_org_mgmt}}" \
    --output "$(cd - >/dev/null && pwd)/{{out_org_mgmt}}" \
    --package-name sesame_idam_org_mgmt_gen \
    --force
  echo "✅ Generated {{out_org_mgmt}}"

# Lint all 4 OpenAPI specs
lint-openapi: lint-openapi-identity-auth lint-openapi-authz-core lint-openapi-api-keys lint-openapi-org-mgmt

lint-openapi-identity-auth:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR."
    exit 1
  fi
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- lint --spec "$(cd - >/dev/null && pwd)/{{spec_identity_auth}}" --fail-on-error

lint-openapi-authz-core:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR."
    exit 1
  fi
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- lint --spec "$(cd - >/dev/null && pwd)/{{spec_authz_core}}" --fail-on-error

lint-openapi-api-keys:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR."
    exit 1
  fi
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- lint --spec "$(cd - >/dev/null && pwd)/{{spec_api_keys}}" --fail-on-error

lint-openapi-org-mgmt:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR."
    exit 1
  fi
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- lint --spec "$(cd - >/dev/null && pwd)/{{spec_org_mgmt}}" --fail-on-error

# Serve identity-auth API with echo handlers (for local try-out)
# Usage: just serve-identity-auth [addr]
serve-identity-auth addr="0.0.0.0:8001":
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR."
    exit 1
  fi
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- serve \
    --spec "$(cd - >/dev/null && pwd)/{{spec_identity_auth}}" \
    --addr {{addr}}

# Serve authz-core API with echo handlers (for local try-out)
# Usage: just serve-authz-core [addr]
serve-authz-core addr="0.0.0.0:8002":
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR."
    exit 1
  fi
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- serve \
    --spec "$(cd - >/dev/null && pwd)/{{spec_authz_core}}" \
    --addr {{addr}}

# Serve api-keys API with echo handlers (for local try-out)
# Usage: just serve-api-keys [addr]
serve-api-keys addr="0.0.0.0:8003":
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR."
    exit 1
  fi
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- serve \
    --spec "$(cd - >/dev/null && pwd)/{{spec_api_keys}}" \
    --addr {{addr}}

# Serve org-mgmt API with echo handlers (for local try-out)
# Usage: just serve-org-mgmt [addr]
serve-org-mgmt addr="0.0.0.0:8004":
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR."
    exit 1
  fi
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- serve \
    --spec "$(cd - >/dev/null && pwd)/{{spec_org_mgmt}}" \
    --addr {{addr}}

# =============================================================================
# Sync specs from BRRTRouter canonical (optional)
# =============================================================================
# Copy canonical OpenAPI from BRRTRouter into this repo (e.g. after upstream changes).
# Then review diff and commit.
#
# Note: These sync only the identity and access-management canonicals.
# After syncing, you MUST manually split the merged spec into the 4 service
# directories (identity-auth, authz-core, api-keys, org-mgmt).
sync-specs-from-brrtrouter:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR."
    exit 1
  fi
  echo "⚠️  After syncing, manually split into 4 service directories:"
  echo "   1. Copy identity-openapi.yaml → openapi/identity-auth/openapi.yaml"
  echo "   2. Extract /api/v1/am/principals/*, /api/v1/am/authorize → openapi/authz-core/"
  echo "   3. Extract /api/v1/am/api-keys/* → openapi/api-keys/"
  echo "   4. Extract /orgs/*, /api/v1/am/applications/* → openapi/org-mgmt/"
  echo "   5. Run: just lint-openapi"
