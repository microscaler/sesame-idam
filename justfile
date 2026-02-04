# Sesame-IDAM justfile
# Repo layout: microservices/idam/{authentication,authorization}; openapi/idam/{authentication,authorization}.
# Consumes shared BRRTRouter tooling (brrtrouter-gen) for codegen, lint, serve.
# Set BRRTRouter_DIR if BRRTRouter is not a sibling repo (e.g. export BRRTRouter_DIR=/path/to/BRRTRouter).

set shell := ["bash", "-uc"]

# BRRTRouter repo path (sibling of seasame-idam by default)
brrtrouter_dir := env_var("BRRTRouter_DIR") or "../BRRTRouter"

# microscaler-supabase side-clone (for Supabase stack). Set SUPABASE_DIR if not a sibling.
supabase_dir := env_var("SUPABASE_DIR") or "../microscaler-supabase"

# OpenAPI spec paths
spec_auth := "openapi/idam/authentication/openapi.yaml"
spec_authorization := "openapi/idam/authorization/openapi.yaml"

# Output dirs for brrtrouter-gen (gen crates live under each microservice)
out_auth := "microservices/idam/authentication/gen"
out_authorization := "microservices/idam/authorization/gen"

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

# Start development environment (Kind cluster + local registry + Tilt)
dev-up:
  #!/usr/bin/env bash
  set -euo pipefail
  echo "🚀 Starting Sesame-IDAM development environment..."
  if [ ! -d tooling/.venv ]; then
    echo "❌ tooling/.venv not found. Run: just init"
    exit 1
  fi
  echo "📦 Creating Kind cluster..."
  kind create cluster --config kind-config.yaml || true
  echo "📦 Setting up local registry (localhost:5001)..."
  tooling/.venv/bin/sesame tilt setup-kind-registry
  echo "⏳ Waiting for cluster to be ready..."
  kubectl wait --for=condition=Ready nodes --all --timeout=300s
  echo "📁 Creating sesame-idam namespace..."
  kubectl apply -f k8s/microservices/namespace.yaml
  echo "💾 Creating PersistentVolumes (data + monitoring)..."
  tooling/.venv/bin/sesame tilt setup-persistent-volumes || true
  echo "📦 Creating data dir on host for PVs (if using extraMounts)..."
  mkdir -p /tmp/sesame-idam-data/postgres /tmp/sesame-idam-data/parquet-lake /tmp/sesame-idam-data/redis /tmp/sesame-idam-data/prometheus /tmp/sesame-idam-data/grafana
  echo "📦 Apply Supabase stack once: just supabase-apply (then start Tilt)"
  echo "🎯 Starting Tilt (loads Redis, tooling; Postgres from microscaler-supabase in namespace data)..."
  tilt up --host=0.0.0.0 --port=10351

# Stop development environment (Kind cluster and Tilt; local registry left running)
dev-down:
  #!/usr/bin/env bash
  set -euo pipefail
  echo "🛑 Stopping Sesame-IDAM development environment..."
  pkill -f "tilt up" 2>/dev/null || true
  kind delete cluster --name sesame-idam 2>/dev/null || true
  echo "✅ Development environment stopped"
  echo "   (Local registry kind-registry is left running. To remove: just dev-down-full)"

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
  @tilt up

# Start with Kind (cluster and namespace must exist)
up-k8s:
  @kubectl apply -f k8s/microservices/namespace.yaml 2>/dev/null || true
  @echo "Starting all services with Tilt (Kubernetes mode)..."
  @tilt up -- --use-kind

# Stop Tilt
down:
  @tilt down

# Show cluster and service status
status:
  #!/usr/bin/env bash
  set -euo pipefail
  echo "Cluster status:"
  kind get clusters 2>/dev/null | grep sesame-idam || echo "No sesame-idam Kind cluster found"
  echo ""
  echo "Pods (sesame-idam):"
  kubectl get pods -n sesame-idam 2>/dev/null || echo "Namespace not found"
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
# Requires BRRTRouter at BRRTRouter_DIR. Run from seasame-idam repo root.

# Regenerate both authentication and authorization gen crates from OpenAPI
gen: gen-auth gen-authorization

# Regenerate authentication (Identity) gen crate from OpenAPI
gen-auth:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR or clone BRRTRouter as a sibling."
    exit 1
  fi
  echo "🔨 Generating authentication (Identity) from {{spec_auth}}..."
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- generate \
    --spec "$(cd - >/dev/null && pwd)/{{spec_auth}}" \
    --output "$(cd - >/dev/null && pwd)/{{out_auth}}" \
    --package-name sesame_idam_authentication_gen \
    --force
  echo "✅ Generated {{out_auth}}"

# Regenerate authorization (Access Management) gen crate from OpenAPI
gen-authorization:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR or clone BRRTRouter as a sibling."
    exit 1
  fi
  echo "🔨 Generating authorization (AM) from {{spec_authorization}}..."
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- generate \
    --spec "$(cd - >/dev/null && pwd)/{{spec_authorization}}" \
    --output "$(cd - >/dev/null && pwd)/{{out_authorization}}" \
    --package-name sesame_idam_authorization_gen \
    --force
  echo "✅ Generated {{out_authorization}}"

# Lint both OpenAPI specs (via BRRTRouter)
lint-openapi: lint-openapi-auth lint-openapi-authorization

# Lint authentication OpenAPI spec
lint-openapi-auth:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR."
    exit 1
  fi
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- lint --spec "$(cd - >/dev/null && pwd)/{{spec_auth}}" --fail-on-error

# Lint authorization OpenAPI spec
lint-openapi-authorization:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR."
    exit 1
  fi
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- lint --spec "$(cd - >/dev/null && pwd)/{{spec_authorization}}" --fail-on-error

# Serve authentication API with echo handlers (for local try-out)
# Usage: just serve-auth [addr]
serve-auth addr="0.0.0.0:8080":
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR."
    exit 1
  fi
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- serve \
    --spec "$(cd - >/dev/null && pwd)/{{spec_auth}}" \
    --addr {{addr}}

# Serve authorization API with echo handlers (for local try-out)
# Usage: just serve-authorization [addr]
serve-authorization addr="0.0.0.0:8081":
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR."
    exit 1
  fi
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- serve \
    --spec "$(cd - >/dev/null && pwd)/{{spec_authorization}}" \
    --addr {{addr}}

# =============================================================================
# Sync specs from BRRTRouter canonical (optional)
# =============================================================================
# Copy canonical OpenAPI from BRRTRouter into this repo (e.g. after upstream changes).
# Then review diff and commit.

sync-specs-from-brrtrouter:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR."
    exit 1
  fi
  cp "{{brrtrouter_dir}}/docs/SPIFFY_mTLS/openapi/identity-openapi.yaml" "{{spec_auth}}"
  cp "{{brrtrouter_dir}}/docs/SPIFFY_mTLS/openapi/access-management-openapi.yaml" "{{spec_authorization}}"
  echo "✅ Copied canonical specs. Restore header comments in openapi files (Sesame-IDAM derived from canonical) then run just lint-openapi"
