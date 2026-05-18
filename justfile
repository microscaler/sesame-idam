# Sesame-IDAM justfile
# Repo layout: 6 services split by access pattern for independent scaling:
#   identity-login-service   — login, register, social, OTP flows (8101)
#   identity-session-service — refresh, OIDC, JWKS (8105)
#   identity-user-mgmt-service — user CRUD, MFA, email/phone (8106)
#   authz-core               — per-request authorization checks (8102)
#   api-keys                 — M2M key management/validation (8103)
#   org-mgmt                 — org lifecycle & SSO admin (8104)
# OpenAPI specs: openapi/{identity-login-service,identity-session-service,identity-user-mgmt-service,authz-core,api-keys,org-mgmt}/openapi.yaml
# Consumes shared BRRTRouter tooling (brrtrouter-gen) for codegen, lint, serve.
# Set BRRTRouter_DIR if BRRTRouter is not a sibling repo (e.g. export BRRTRouter_DIR=/path/to/BRRTRouter).

set shell := ["bash", "-uc"]

# BRRTRouter repo path (sibling of seasame-idam)
# Override with: BRRTRouter_DIR=/path/to/BRRTRouter just lint-openapi
brrtrouter_dir := "../BRRTRouter"

# microscaler-supabase side-clone (for Supabase stack)
# Override with: SUPABASE_DIR=/path/to/microscaler-supabase just supabase-apply
supabase_dir := "../microscaler-supabase"

# OpenAPI spec paths (6 services split by access pattern)
spec_identity_login     := "openapi/idam/identity-login-service/openapi.yaml"
spec_identity_session   := "openapi/idam/identity-session-service/openapi.yaml"
spec_identity_user_mgmt := "openapi/idam/identity-user-mgmt-service/openapi.yaml"
spec_authz_core         := "openapi/idam/authz-core/openapi.yaml"
spec_api_keys           := "openapi/idam/api-keys/openapi.yaml"
spec_org_mgmt           := "openapi/idam/org-mgmt/openapi.yaml"

# Output dirs for brrtrouter-gen (gen crates live under each microservice)
out_identity_login      := "microservices/idam/identity-login-service/gen"
out_identity_session    := "microservices/idam/identity-session-service/gen"
out_identity_user_mgmt  := "microservices/idam/identity-user-mgmt-service/gen"
out_authz_core          := "microservices/idam/authz-core/gen"
out_api_keys            := "microservices/idam/api-keys/gen"
out_org_mgmt            := "microservices/idam/org-mgmt/gen"
# Default recipe to display help
default:
  @just --list --unsorted

# =============================================================================
# Database / Test Environment Variables
# =============================================================================
# Local dev: postgres forwarded to localhost:5432 (see `just port-forward`)
# DB_HOST, DB_PORT (default 5432), DB_NAME (default sesame_idam)
DATABASE_URL := "postgres://sesame_idam:***@127.0.0.1:5432/sesame_idam"

# =============================================================================
# Testing (nextest)
# =============================================================================
# Plain `cargo test` does **not** match what CI / this repo calls the "workspace suite": it ignores
# `cargo nextest`, `.config/nextest.toml` (timeouts, `db_integration_suite` mutex), and the filters below.
#
# Nextest quick reference:
#   nt / nextest-test     — workspace nextest; excludes db_integration_suite binary (fast loop)
#   nt-workspace          — CI-parity workspace (all members except db_integration_suite binary)
#   nt-db-suite           — serial DB integration tests only (shared Postgres safe)
#   nt-complete           — nextest-test then nt-db-suite (typical local: all workspace members)
#   nt-ci-parity / nt-full — nt-workspace then nt-db-suite (matches CI)
#   nt-verbose            — same as nt but with --no-capture (full stdout/stderr)
#   nt-unit               — unit tests only (same filter as nextest-test)
#
# Coverage ladder (pick one path):
#   • Fast loop (~`just nt`):                    `just nt` or `just nextest-test`
#   • CI workspace step only:                    `just nt-workspace`
#   • Serial DB integration (`db_integration_suite`): `just nt-db-suite`
#   • Typical "all Rust tests" locally:           `just nt-complete` (= nt + nt-db-suite)
#   • **Full CI parity** (workspace + db suite): `just nt-ci-parity` or `just nt-full`

# Library unit tests only (`cargo test --lib`). For broad coverage use `just nt`, `just nt-complete`, or `just nt-ci-parity`.
test: test-unit

# Run unit tests
test-unit:
    @echo "🧪 Running unit tests..."
    @DATABASE_URL={{DATABASE_URL}} TEST_DATABASE_URL={{DATABASE_URL}} cargo test --manifest-path microservices/Cargo.toml --lib --no-fail-fast

# Run unit tests with output
test-unit-verbose:
    @echo "🧪 Running unit tests (verbose)..."
    @DATABASE_URL={{DATABASE_URL}} TEST_DATABASE_URL={{DATABASE_URL}} cargo test --manifest-path microservices/Cargo.toml --lib -- --nocapture --no-fail-fast

# Workspace nextest: excludes db_integration_suite for speed (use nt-db-suite / nt-complete).
# Note: db_integration_suite is safe in parallel with other *packages* — nextest test-group `lifeguard-shared-postgres`
# serializes tests inside that binary only (see .config/nextest.toml).
nextest-test:
    @echo "🧪 Running tests with nextest (excluding DB-heavy integration binaries)..."
    @DATABASE_URL={{DATABASE_URL}} TEST_DATABASE_URL={{DATABASE_URL}} cargo nextest run --manifest-path microservices/Cargo.toml --workspace --all-features --fail-fast --retries 1

alias nt := nextest-test

# Broadest automated suite aligned with CI (workspace including db_integration_suite).
alias nt-full := nt-ci-parity

# CI-parity workspace nextest (same filter as .github/workflows/ci.yaml "Run workspace tests").
# Includes all members; requires DATABASE_URL.
nt-workspace:
    @echo "🧪 Running workspace nextest (CI selection: all members)..."
    @DATABASE_URL={{DATABASE_URL}} TEST_DATABASE_URL={{DATABASE_URL}} cargo nextest run --manifest-path microservices/Cargo.toml --workspace --all-features --profile ci

# Run tests with nextest (no capture - passes through stdout/stderr directly)
nt-verbose:
    @echo "🧪 Running tests with nextest (no capture - full output)..."
    @DATABASE_URL={{DATABASE_URL}} TEST_DATABASE_URL={{DATABASE_URL}} cargo nextest run --manifest-path microservices/Cargo.toml --workspace --all-features --no-capture

# Same as `nt-workspace` (alias for discoverability)
nt-ci:
    @just nt-workspace

# Run unit tests only with nextest (same selection as nextest-test)
nt-unit:
    @echo "🧪 Running tests with nextest (excluding DB-heavy integration binaries)..."
    @DATABASE_URL={{DATABASE_URL}} TEST_DATABASE_URL={{DATABASE_URL}} cargo nextest run --manifest-path microservices/Cargo.toml --workspace --all-features --fail-fast --retries 1

# DB integration suite: serial (shared Postgres safe).
nt-db-suite:
    @echo "🧪 Running DB integration tests (serial profile)..."
    @DATABASE_URL={{DATABASE_URL}} TEST_DATABASE_URL={{DATABASE_URL}} cargo nextest run --manifest-path microservices/Cargo.toml --workspace --all-features --profile db-serial --config-file .config/nextest.toml

alias nt-db := nt-db-suite
# Same as `nt-db-suite` (CI step name / copy-paste alias)
alias db-integration-suite := nt-db-suite

# Verbose output for db suite only
nt-db-suite-verbose:
    @echo "🧪 Running DB integration tests (serial, no-capture)..."
    @DATABASE_URL={{DATABASE_URL}} TEST_DATABASE_URL={{DATABASE_URL}} cargo nextest run --manifest-path microservices/Cargo.toml --workspace --all-features --profile db-serial --config-file .config/nextest.toml --no-capture

# Typical local run: fast workspace (no cluster integration crate) + serial DB suite
nt-complete: nextest-test nt-db-suite
    @echo "✅ Workspace + db_integration_suite complete."

# Matches CI order: workspace nextest + serial db_integration_suite
nt-ci-parity: nt-workspace nt-db-suite
    @echo "✅ CI-parity test run complete (workspace + db_integration_suite)."

# Run integration tests with nextest
nt-integration:
    @echo "🧪 Running integration tests with nextest..."
    @echo "⚠️  Note: These tests require a running database connection"
    @DATABASE_URL={{DATABASE_URL}} TEST_DATABASE_URL={{DATABASE_URL}} cargo nextest run --manifest-path microservices/Cargo.toml --workspace --all-features

# Run tests with standard cargo (fallback)
test-cargo:
    @echo "🧪 Running tests with cargo..."
    @DATABASE_URL={{DATABASE_URL}} TEST_DATABASE_URL={{DATABASE_URL}} cargo test --manifest-path microservices/Cargo.toml --all -- --nocapture

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

# Lint workspace (clippy with pedantic mode, JSF-aligned thresholds from clippy.toml)
# Excludes gen/ crates — they are auto-generated by brrtrouter-gen and cannot be linted.
# Lints only the impl crates + database + audit + migrator.
lint-rust:
    @echo "🔍 Linting Rust workspace..."
    @cd microservices && cargo clippy --all-targets --all-features \
      -p sesame_idam_database \
      -p sesame-audit \
      -p sesame_idam_migrator \
      -p sesame_idam_identity_login_service \
      -p sesame_idam_identity_session_service \
      -p sesame_idam_identity_user_mgmt_service \
      -p sesame_idam_authz_core \
      -p sesame_idam_api_keys \
      -p sesame_idam_org_mgmt \
      -- -D warnings -W clippy::pedantic

# When microservices/gen are added: cargo build --release --workspace
# build:
#   cargo build --release --workspace

# =============================================================================
# BRRTRouter codegen (shared tooling)
# =============================================================================
# BRRTRouter codegen (shared tooling)
# =============================================================================

# Regenerate all 6 services from OpenAPI
gen: gen-identity-login gen-identity-session gen-identity-user-mgmt gen-authz-core gen-api-keys gen-org-mgmt

# Regenerate identity-login-service (login, register, social, OTP) gen crate
gen-identity-login:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR or clone BRRTRouter as a sibling."
    exit 1
  fi
  echo "🔨 Generating identity-login-service from {{spec_identity_login}}..."
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- generate \
    --spec "$(cd - >/dev/null && pwd)/{{spec_identity_login}}" \
    --output "$(cd - >/dev/null && pwd)/{{out_identity_login}}" \
    --package-name sesame_idam_identity_login_service_gen \
    --force
  echo "✅ Generated {{out_identity_login}}"

# Regenerate identity-session-service (refresh, OIDC, JWKS) gen crate
gen-identity-session:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR or clone BRRTRouter as a sibling."
    exit 1
  fi
  echo "🔨 Generating identity-session-service from {{spec_identity_session}}..."
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- generate \
    --spec "$(cd - >/dev/null && pwd)/{{spec_identity_session}}" \
    --output "$(cd - >/dev/null && pwd)/{{out_identity_session}}" \
    --package-name identity_session_service_service_api \
    --force
  echo "✅ Generated {{out_identity_session}}"

# Regenerate identity-user-mgmt-service (user CRUD, MFA, email/phone) gen crate
gen-identity-user-mgmt:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR or clone BRRTRouter as a sibling."
    exit 1
  fi
  echo "🔨 Generating identity-user-mgmt-service from {{spec_identity_user_mgmt}}..."
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- generate \
    --spec "$(cd - >/dev/null && pwd)/{{spec_identity_user_mgmt}}" \
    --output "$(cd - >/dev/null && pwd)/{{out_identity_user_mgmt}}" \
    --package-name sesame_idam_identity_user_mgmt_service_gen \
    --force
  echo "✅ Generated {{out_identity_user_mgmt}}"

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

# Lint all 6 OpenAPI specs
lint-openapi: lint-openapi-identity-login lint-openapi-identity-session lint-openapi-identity-user-mgmt lint-openapi-authz-core lint-openapi-api-keys lint-openapi-org-mgmt

lint-openapi-identity-login:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR."
    exit 1
  fi
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- lint --spec "$(cd - >/dev/null && pwd)/{{spec_identity_login}}" --fail-on-error

lint-openapi-identity-session:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR."
    exit 1
  fi
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- lint --spec "$(cd - >/dev/null && pwd)/{{spec_identity_session}}" --fail-on-error

lint-openapi-identity-user-mgmt:
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR."
    exit 1
  fi
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- lint --spec "$(cd - >/dev/null && pwd)/{{spec_identity_user_mgmt}}" --fail-on-error

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

# Serve identity-login-service API with echo handlers (for local try-out)
# Usage: just serve-identity-login [addr]
serve-identity-login addr="0.0.0.0:8101":
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR."
    exit 1
  fi
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- serve \
    --spec "$(cd - >/dev/null && pwd)/{{spec_identity_login}}" \
    --addr {{addr}}

# Serve identity-session-service API with echo handlers (for local try-out)
# Usage: just serve-identity-session [addr]
serve-identity-session addr="0.0.0.0:8105":
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR."
    exit 1
  fi
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- serve \
    --spec "$(cd - >/dev/null && pwd)/{{spec_identity_session}}" \
    --addr {{addr}}

# Serve identity-user-mgmt-service API with echo handlers (for local try-out)
# Usage: just serve-identity-user-mgmt [addr]
serve-identity-user-mgmt addr="0.0.0.0:8106":
  #!/usr/bin/env bash
  set -euo pipefail
  if [ ! -d "{{brrtrouter_dir}}" ]; then
    echo "❌ BRRTRouter not found at {{brrtrouter_dir}}. Set BRRTRouter_DIR."
    exit 1
  fi
  cd "{{brrtrouter_dir}}" && cargo run --bin brrtrouter-gen -- serve \
    --spec "$(cd - >/dev/null && pwd)/{{spec_identity_user_mgmt}}" \
    --addr {{addr}}

# Serve authz-core API with echo handlers (for local try-out)
# Usage: just serve-authz-core [addr]
serve-authz-core addr="0.0.0.0:8102":
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
serve-api-keys addr="0.0.0.0:8103":
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
serve-org-mgmt addr="0.0.0.0:8104":
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
  echo "⚠️  After syncing, manually split into 6 service directories:"
  echo "   1. Split identity login/registration/OTP → openapi/identity-login-service/"
  echo "   2. Split refresh/OIDC/JWKS → openapi/identity-session-service/"
  echo "   3. Split user CRUD/MFA/email-phone → openapi/identity-user-mgmt-service/"
  echo "   4. Extract /authz/principals/*, /authz/authorize → openapi/authz-core/"
  echo "   5. Extract /api-keys/* → openapi/api-keys/"
  echo "   6. Extract /organizations/*, /applications/* → openapi/org-mgmt/"
  echo "   7. Run: just lint-openapi"

# =============================================================================
# Systemd Service Management (tilt-sesame-idam)
# =============================================================================
# The Tilt service is managed via systemd user units:
#   ~/.config/systemd/user/tilt-sesame-idam.service
#
# Usage:
#   just tilt-up     — start the Tilt systemd service (or use: systemctl --user start tilt-sesame-idam)
#   just tilt-down   — stop the Tilt systemd service (or use: systemctl --user stop tilt-sesame-idam)
#   just tilt-log    — tail the Tilt service journal (or use: journalctl --user -u tilt-sesame-idam -f)
#   just tilt-status — check Tilt service status (or use: systemctl --user status tilt-sesame-idam)

# Start Tilt via systemd (loads on login via WantedBy=default.target)
tilt-up:
  @echo "Starting sesame-idam Tilt via systemd..."
  @systemctl --user start tilt-sesame-idam.service
  @sleep 2
  @echo "Tilt UI: http://localhost:10351"

# Stop Tilt via systemd
tilt-down:
  @echo "Stopping sesame-idam Tilt via systemd..."
  @systemctl --user stop tilt-sesame-idam.service
  @pkill -f "tilt up" 2>/dev/null || true
  @echo "Tilt stopped"

# Tail Tilt service logs
tilt-log:
  @journalctl --user -u tilt-sesame-idam.service -f

# Check Tilt service status
tilt-status:
  @systemctl --user status tilt-sesame-idam.service

# Reload systemd after unit file changes (e.g. editing the .service file)
tilt-reload:
  @echo "Reloading systemd user daemon..."
  @systemctl --user daemon-reload
  @echo "Done — Tilt unit reloaded"
