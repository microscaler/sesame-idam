# Sesame-IDAM Agent Notes

**Purpose:** Quick reference for agentic coding assistants working on the Sesame-IDAM repository.

## Repository role

Sesame-IDAM is an **IDAM component for microservices**, not a standalone SaaS IDAM product. The repo is laid out like **RERP** with a `microservices/` directory. All previous Rust code has been removed; implementations will be added as microservices under `microservices/idam/`.

## Layout (RERP-style)

- **microservices/** — Workspace crates (see `microservices/Cargo.toml`). No members yet; add `idam/authentication/gen`, `idam/authentication/impl`, `idam/authorization/gen`, `idam/authorization/impl` when implementing.
- **microservices/idam/** — IDAM domain. Two microservices identified so far:
  - **authentication** — Identity, login, refresh, logout, token exchange, register, sessions, JWKS/OIDC. See `microservices/idam/authentication/README.md`.
  - **authorization** — Access Management: apps, roles, permissions, principal/effective, authorize. See `microservices/idam/authorization/README.md`.
  - Further components may be added under `idam/` as needed.
- **openapi/idam/** — OpenAPI specs per microservice. Each has `openapi.yaml` (derived from BRRTRouter canonical). Canonical: `BRRTRouter/docs/SPIFFY_mTLS/openapi/` (identity-openapi.yaml, access-management-openapi.yaml). See `openapi/idam/README.md`.
- **specs/** — Legacy/openapi.yaml (pre-pivot); may be retired or aligned with openapi/idam.
- **Cargo.toml** (root) — Workspace with `members = ["microservices"]`.

## Tooling (same guard rails as RERP)

Sesame-IDAM has a **tooling/** package (Python) with the same strict **ruff** and **pre-commit** guard rails as RERP:

- **Ruff:** Same select/ignore and mccabe max-complexity 20 as RERP (see `tooling/pyproject.toml`).
- **Pre-commit:** `just qa` (lint + format-check + pytest) and check for forbidden empty print statements (`.pre-commit-config.yaml`).
- **Justfile:** `just init`, `just build-tooling`, `just venv`, `just lint`, `just format`, `just format-check`, `just qa`, `just install-hooks`, `just lint-fix`, `just lint-unused-imports`.

Run **`just init`** once, then **`just qa`** before commit; **`just install-hooks`** to install pre-commit hooks. CI runs **tooling-qa** (just init + just qa).

## Development environment (same DX as RERP)

- **Kind:** Cluster name `sesame-idam` (context `kind-sesame-idam`). Config: `kind-config.yaml` with port mappings (PostgreSQL 5433, Redis 6379, Prometheus 9091, Grafana 3002) and extraMounts (`/tmp/sesame-idam-data` → `/mnt/sesame-idam-data` for PV data).
- **Namespace:** `sesame-idam` (created at dev-up via `k8s/microservices/namespace.yaml`).
- **Local registry:** `localhost:5001` (container `kind-registry`). Setup: `sesame tilt setup-kind-registry`.
- **Data components:** **Supabase stack** (Postgres, postgres-meta, parquet-lake) is **externalised** to **microscaler-supabase** (side clone at `../microscaler-supabase`). Apply once: `just supabase-apply` (applies `k8s/overlays/seasame-idam` from microscaler-supabase; creates namespace `data`, postgres, etc.). **Redis** remains in-repo: `k8s/data/persistent-volumes.yaml` (Redis PV only), `k8s/data/redis.yaml`. Tilt loads namespace, Redis PV, Redis only; `just port-forward` forwards postgres (namespace `data`) and redis (namespace `sesame-idam`).
- **Helm app config:** `app.config.database` host `postgres.data.svc.cluster.local` (postgres in namespace `data`), name/user `postgres`; `app.config.redis` host `redis`, port 6379.
- **Tilt:** Port 10351. Run `just dev-up` (Kind + registry + namespace + PVs + Tilt). Run `just supabase-apply` once to deploy Supabase stack (namespace `data`), then `just port-forward` for postgres + redis.
- **Justfile:** `supabase-apply` (apply Supabase from microscaler-supabase overlay), `dev-up`, `dev-down`, `dev-down-full`, `setup`, `teardown`, `up`, `up-k8s`, `down`, `status`, `port-forward`.
- **Docker:** `docker/microservices/Dockerfile.template` for authentication/authorization (when gen+impl exist).
- **Helm:** `helm/sesame-idam-microservice/` with values for authentication (8001) and authorization (8002); configmap includes database and redis (same as RERP).
- **Tiltfile:** Tooling + data (Supabase postgres, Redis) live; microservice deploy wired when gen+impl exist.

## Build and deploy (shared BRRTRouter tooling)

Sesame-IDAM consumes **BRRTRouter** for codegen, lint, and serve (same pattern as RERP). Set `BRRTRouter_DIR` if BRRTRouter is not a sibling repo (default `../BRRTRouter`).

| Command | Description |
|--------|-------------|
| `just gen` | Regenerate both authentication and authorization gen crates from OpenAPI (writes to `microservices/idam/*/gen`) |
| `just gen-auth` | Regenerate authentication (Identity) gen crate only |
| `just gen-authorization` | Regenerate authorization (AM) gen crate only |
| `just lint-openapi` | Lint both OpenAPI specs via brrtrouter-gen |
| `just serve-auth [addr]` | Serve authentication API with echo handlers (default `0.0.0.0:8080`) |
| `just serve-authorization [addr]` | Serve authorization API with echo handlers (default `0.0.0.0:8081`) |
| `just sync-specs-from-brrtrouter` | Copy canonical specs from BRRTRouter; then restore Sesame-IDAM header comments and run `just lint-openapi` |

After adding `gen/` and `impl/` crates, extend the justfile with build/docker/Tilt targets (like RERP) that use BRRTRouter tooling or delegate to a future Python tooling layer.

## Key context

- **Audit and transformation:** `BRRTRouter/docs/SPIFFY_mTLS/Sesame_IDAM_Audit_and_Transformation_Analysis.md` — pivot rationale, gap analysis, transformation roadmap.
- **Target design:** Identity + Access Management as in `BRRTRouter/docs/SPIFFY_mTLS/Generic_Identity_Service_IDAM_Design.md`, `Generic_Access_Management_Service_Design.md`, and the OpenAPI specs there.
- **Implementing a microservice:** Follow RERP pattern: `gen/` (BRRTRouter-generated from OpenAPI) + `impl/` (binary + lifeguard/persistence). Register both in `microservices/Cargo.toml`.

## Archive

Pre-pivot state (SaaS IDAM with Sea-ORM and single Rust crate) is preserved on branch `archive/saas-idam-pre-microservice-pivot`, tag `archive/saas-idam-2025-02-02`, and repo `git@github.com:microscaler/sesame-idam-archived.git`.
