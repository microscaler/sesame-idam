# Sesame-IDAM — Agent Rules

> **Desktop dev environment** — before doing anything in this repo, read the
> Microscaler-wide topology brief. It explains that you are on a Mac but the
> code lives on `ms02` (NFS), where commands execute for this environment, how
> the Kind cluster and vLLM fit in, and the network constraints behind the SSH
> tunneling. Do not duplicate its contents here — link to it. If reality drifts,
> fix the canonical doc, not this copy.
>
> - GitHub: [`cylon-local-infra/docs/desktop-dev-environment.md`](https://github.com/microscaler/cylon-local-infra/blob/main/docs/desktop-dev-environment.md)
> - On ms02 NFS: `~/Workspace/microscaler/cylon-local-infra/docs/desktop-dev-environment.md`

## CRITICAL: Microscaler Dependencies

**We do NOT publish to crates.io. We consume from microscaler forks.**

When analyzing Cargo.toml dependencies, NEVER assume crates.io is the source for any dependency that has a microscaler fork. The crates.io versions are stale or abandoned.

### Microscaler Fork Inventory

These repos exist in `microscaler/` as forks with custom changes:

| Fork Repo | Upstream | Purpose |
|-----------|----------|---------|
| `microscaler/may` | Xudong-Huang/may | Core stackful coroutine runtime. Foundation of BRRTRouter. |
| `microscaler/may_minihttp` | Xudong-Huang/may_minihttp | Mini HTTP server. Fork until PR #21 merged upstream. Provides `TestClient`. |
| `microscaler/may_postgres` | (no direct upstream) | Postgres driver for may coroutines. Custom features. |
| `microscaler/generator-rs` | Xudong-Huang/generator-rs | Coroutine generator. Patched via `[patch.crates-io]` for Rust 1.90 macOS thread-local bug. |
| `microscaler/mayfly` | (no upstream) | Separate project. |

### Dependency Resolution Rules

- `may` → `git = "https://github.com/microscaler/may.git"` (NOT crates.io)
- `may_minihttp` → `git = "https://github.com/microscaler/may_minihttp.git", branch = "integration/microscaler-fork"` (NOT crates.io — forks add `TestClient`)
- `may_postgres` → `git = "https://github.com/microscaler/may_postgres.git", branch = "master"` (NOT crates.io)
- `generator` → patched via `[patch.crates-io]` to `git = "https://github.com/microscaler/generator-rs.git"`
- `may_http` → `git = "https://github.com/rust-may/may_http.git"` (upstream fork, not microscaler-owned)
- `lifeguard` → local path `../../lifeguard` (sibling repo) — **see below**
- `brrtrouter` → local path `../../BRRTRouter` (sibling repo) — **see below**

**Never guess. If you see `may`, `may_minihttp`, `may_postgres`, `generator`, or any microscaler-related crate, verify the source by checking the Cargo.toml — never assume crates.io.**

### Why Some Deps Use `path =` Instead of `git =`

**lifeguard** and **BRRTRouter** are sibling repos that share remotes — they ARE pushed remotely and CI switches them to git pins. They use `path =` locally for co-development convenience:

- When editing a dependency repo (lifeguard, BRRTRouter) and a consumer (hauliage, sesame-idam) simultaneously, path deps avoid the commit-push-update dance. With a path dep, you see changes compile immediately. With a git dep, you have to commit, push, update `Cargo.toml`, run `cargo update`.
- CI validates the other direction: git dep pins verify a consumer actually works against the *published* version, catching "works locally but not on published version" drift.

This is not about NFS, shared mounts, or missing remotes. lifeguard and BRRTRouter have full remotes, CI uses git pins, and they are actively co-developed across all repos.

---

Strict operational rules for AI assistants and humans working in this repository. **Knowledge about how Sesame-IDAM works is in [`docs/llmwiki/`](./docs/llmwiki/), not here.** This file only holds rules the agent must obey. See [`CONTRIBUTING.md`](./CONTRIBUTING.md) for the completion gates — every story requires compilation, pedantic linting, unit tests, and BDD E2E tests before it is considered done. No exceptions.

---

## Before you do anything

1. Load the `systemd-tilt-services` skill — it contains critical Tilt workflow rules (systemd-only, never run `tilt up` directly, how to check pod status, etc.).
2. Read [`docs/llmwiki/README.md`](./docs/llmwiki/README.md) — the wiki entry point. It redirects to `SCHEMA.md` + `index.md` + `log.md`.
3. Read [`docs/llmwiki/index.md`](./docs/llmwiki/index.md) — the wiki index. Scan it to identify what pages are relevant to your task.
4. Tail [`docs/llmwiki/log.md`](./docs/llmwiki/log.md) for recent context.
5. Read **only** the wiki pages identified in step 3. Drill into linked pages only when the wiki flags drift or a gap.

**Key principle: Read index.md, identify what you need, read only those pages. Never read the whole wiki.** Loading all wiki pages into context for a single-file change wastes tokens and buries relevant information under noise.

Working on identity-login-service entity? Read `entities/entity-*.md` for that service's entities and `topics/lifeguard-schema-authoring.md`. Skip authz-core and org-mgmt pages.

---

## Documentation layout

`docs/` is organized into wiki pages and design documents:

| Path | What lives here |
|---|---|
| [`docs/llmwiki/`](./docs/llmwiki/) | Living, LLM-maintained knowledge base — start here. |
| [`docs/design-doc.md`](./docs/design-doc.md) | System-level design (architecture, data model, API surface). Keep current. |
| [`docs/service-topology-design.md`](./docs/service-topology-design.md) | Service split rationale and scaling profiles. |
| [`docs/sesame-idam-complete.md`](./docs/sesame-idam-complete.md) | Vision, developer contract, integration patterns. |
| [`docs/Epics/INDEX.md`](./docs/Epics/INDEX.md) | 9-epic implementation plan with 44 stories across 9 dirs. Status column tracks design vs implementation state. |

Forward-looking planning (`PRD_*.md`, `ROADMAP.md`) stays at the `docs/` root.
Epics and stories are in `docs/Epics/{N}-{name}/` (e.g., `docs/Epics/01-asymmetric-jwks/`). Each epic dir contains an overview doc and a `stories/` subdir with individual story files. INDEX.md is the canonical master index.

---

## Repo shape

Six Rust microservices, each with `gen/` (BRRTRouter-generated from OpenAPI) + `impl/` (binary + lifeguard/persistence). Total: **119 endpoints, 26 tags**.

All services listen on ClusterIP **:8080** in-cluster (service identity is the
Kubernetes Service name). Optional Tilt host port-forwards for isolated debug:
login `8101:8080`, session `8105:8080`.

| Service | Path | Access Pattern | Endpoints |
|---------|------|----------------|-----------|
| identity-login-service | `microservices/idam/identity-login-service/` | HIGH — login, register, social OAuth, OTP, passwordless | 20 |
| identity-session-service | `microservices/idam/identity-session-service/` | HIGH — refresh, OIDC, JWKS, step-up, impersonation, MCP | 13 |
| identity-user-mgmt-service | `microservices/idam/identity-user-mgmt-service/` | MEDIUM — user CRUD, MFA, email/phone, passwordless | 25 |
| authz-core | `microservices/idam/authz-core/` | EXTREME — every consumer API request | 4 |
| api-keys | `microservices/idam/api-keys/` | HIGH — M2M key validation, archiving | 10 |
| org-mgmt | `microservices/idam/org-mgmt/` | LOW — org lifecycle, SSO/SCIM, webhooks, SCIM | 34 |

OpenAPI specs: `openapi/idam/{service}/openapi.yaml` (6 directories, no canonical/merged spec).
Workspace: `microservices/Cargo.toml` (all 12 crates registered: gen+impl for each service).
Shared tooling: `tooling/` (Python, ruff, pre-commit).

---

## Build commands

All commands from repo root unless noted.

### Tooling (Python)

```
just init              # Create .venv and install tooling
just qa                # lint + format-check + tests (run before commit)
just lint              # ruff check tooling/
just format            # ruff format tooling/
just install-hooks     # Install pre-commit hooks
```

### Codegen

```
just gen                # Regenerate all 6 services from OpenAPI
just gen-identity-login # Single service codegen
just lint-openapi       # Lint all 6 OpenAPI specs via brrtrouter-gen
just sync-specs-from-brrtrouter  # Copy canonical specs from BRRTRouter
```

### Build

| Build commands |
|---|---|
| `cargo check --workspace` | Build check from microservices/ |
| `cargo build --workspace` | Build from microservices/ |
| `cargo test --workspace` | Test from microservices/ |

### Testing (nextest)

The workspace uses `cargo nextest` for test execution with configuration in `.config/nextest.toml`.

| Command | What it does |
|---------|-------------|
| `just nt` / `just nextest-test` | Fast loop — workspace tests with `--fail-fast --retries 1` |
| `just nt-workspace` | CI-parity workspace (full `--profile ci`) |
| `just nt-verbose` | Same as nt but `--no-capture` (full stdout/stderr) |
| `just nt-unit` | Unit tests only (same filter as nextest-test) |
| `just nt-complete` | nt + nt-db-suite (typical local: all workspace members) |
| `just nt-ci-parity` | CI-parity: nt-workspace + nt-db-suite |
| `just nt-integration` | Integration tests with nextest |
| `just test-cargo` | Fallback: plain `cargo test --all -- --nocapture` |

**CRITICAL: `just nt` must pass before committing.** All 48+ tests must compile and run clean. Empty `common/mod.rs` files (doc comments only, no actual code) will fail with "expected item after doc comment" — use `//` comments instead of `///` when there's no item to document. Smoke tests must use `#[test]`, not `#[rstest_bdd::bdd]` (rstest-bdd v0.5.0 does not export `bdd` or `Scenario`).

### Dev environment

```
just dev-up          # shared-k8s cluster + Tilt (port 10351)
just dev-down        # Stop Tilt
just supabase-apply  # Apply Supabase stack once (namespace: data)
just port-forward    # Forward postgres + redis (legacy; prefer LAN proxy below)
just tilt-up         # Start Tilt via systemd
just tilt-log        # Tail Tilt logs
```

### Build and deploy on ms02 — remote Tilt from Mac

**Rule**: On desktop dev (Mac + ms02 NFS), **run `cargo check` / `cargo test` on ms02** via `ssh ms02 'source ~/.cargo/env && cd ~/Workspace/microscaler/sesame-idam/microservices && …'`. Mac-local `cargo` is unreliable (toolchain / `ring` / NFS latency).

Tilt for this repo: **systemd `tilt-sesame-idam.service`**, port **10351**, host `0.0.0.0`.

| Action | Command |
|--------|---------|
| Trigger rebuild | `tilt trigger identity-session-service --host 192.168.1.189 --port 10351` |
| Trigger + wait | `cd ../shared-k8s-cluster && just tilt-remote-cycle sesame identity-session-service` |
| Tail build logs | `just tilt-remote-logs sesame identity-session-service` |
| Tilt UI | `http://tilt-sesame.dev.microscaler.local/` |
| BDD (token lifecycle, etc.) | On ms02 with `TEST_DB_HOST=192.168.1.189 TEST_DB_PORT=5433 REDIS_URL=redis://192.168.1.189:6390` |

Do **not** run `tilt up` on Mac. Use systemd on ms02 or the remote trigger recipes above.

Authority: [`../shared-k8s-cluster/docs/remote-tilt-workflow.md`](../shared-k8s-cluster/docs/remote-tilt-workflow.md).

---

## Tenancy & Isolation

**CRITICAL: Sesame-IDAM uses a two-level hierarchy: Tenant (isolation boundary) and Application (logical grouping).**

*   **Tenant = Hard-Segment isolation boundary.** Each customer (e.g., `hauliage`, `rerp`) is a `Tenant`. The `X-Tenant-ID` header maps to the Tenant ID. Data for `Tenant A` is never visible to `Tenant B` — **zero bleed enforced at database level**.
*   **Application = Logical grouping within Tenant.** A tenant can have multiple applications (e.g., hauliage has hauliage-web, hauliage-api, hauliage-admin, hauliage-mobile). Applications share the same tenant data — they are not isolation boundaries.
*   **No Shared Users across Tenants:** Users are strictly scoped to a single `tenant_id`. `alice@corp.com` on `Tenant A` and `alice@corp.com` on `Tenant B` are completely different, unrelated users. No cross-tenant identity exists.
*   **`X-Tenant-ID` Header:** Every API request must include the `X-Tenant-ID` header (or be authenticated via a tenant-scoped API key). This maps to the `tenant_id` in the system.
*   **Database Partitioning:**
    *   **SaaS Model:** Single PostgreSQL schema shared by all tenants. Isolation is enforced via the `tenant_id` column on every major table (`users`, `orgs`, `api_keys`).
    *   **Self-Hosted Model:** The `sesame_idam` database/schema is isolated from the tenant's business logic (e.g., `app` schema) to prevent table name collisions.
*   **Zero Bleed:** Enforced at three layers:
    1.  **Application layer:** BRRTRouter middleware extracts `tenant_id` from `X-Tenant-ID` header, appends `WHERE tenant_id = ?` to all queries.
    2.  **Database layer:** Lifeguard's base executors inject validated `SessionContext` using the
        transaction-local Sesame RLS contract. There is no separate `SesameExecutor`.
    3.  **RLS policies:** PostgreSQL policies enforce tenant and active-organization ownership as a failsafe.

## Core rules the agent must obey

Each rule points at the authoritative source. Open the source when the rule is ambiguous.

### 1. Never edit generated code under `gen/` — protect impl controllers

`microservices/<service>/gen/` is regenerated by `brrtrouter-gen` from `openapi/<service>/openapi.yaml`. Any edit will be clobbered on next regen. Fix the OpenAPI spec instead.

**Impl controllers** (`microservices/idam/<service>/impl/src/controllers/*.rs`) hold business logic. Once you replace a stub with real code, add this as the **first line**:

```rust
// BRRTRouter: user-owned
```

Without a sentinel, `brrtrouter-gen generate-stubs --force` **will overwrite your implementation** with an empty template stub.

Also recognized: `// BRRTROUTER_USER_OWNED`, `// Implemented`.

| Command | Behaviour |
|---------|-----------|
| `generate-stubs` (no flags) | Create missing stubs only; skip existing files |
| `generate-stubs --sync` | Patch signature / `Response` on sentinel-protected files after OpenAPI schema changes |
| `generate-stubs --force` | Overwrite unprotected stubs only; skips files with a sentinel |

Authority: BRRTRouter `src/generator/project/generate.rs`, BRRTRouter `AGENTS.md` §1b.

### 2. Schema changes via Lifeguard entities only

Edit `microservices/<service>/impl/src/models/*.rs`, then regenerate with the migrator. Do **not** hand-write SQL migrations.

### 3. OpenAPI spec is source of truth for request/response shapes

Every UI-dynamic field must be in the spec. BRRTRouter's parser silently drops unrecognized fields.

### 4. Shared schemas are duplicated per spec

Each OpenAPI spec must be self-contained for codegen. Shared schemas (User, UserProfile, Org) are duplicated in `components/schemas` of each consuming spec. This is intentional.

### 5. Only cross-service dependency: login → authz-core

`identity-login-service` calls `authz-core` `/principal/effective` at login time for JWT claim enrichment. After the JWT is issued, it is self-contained. All other services are fully independent.

### 6. Native PostgreSQL via Kind

Tests bind directly to the shared Kind PostgreSQL (namespace `data`, forwarded to localhost:5432). Do not introduce testcontainers.

### 7. Code style + test discipline

- Run `cargo fmt` + `just lint-rust` before committing Rust changes.
  - `just lint-rust` runs clippy with `-D warnings -W clippy::pedantic` (pedantic mode is mandatory for security-critical code).
  - Numeric thresholds are JSF-aligned (same as BRRTRouter / lifeguard) in `clippy.toml`: `stack-size-threshold=512000`, `cognitive-complexity-threshold=30`, `too-many-arguments-threshold=8`.
- Run `just qa` before committing Python changes in `tooling/`.
- Maintain test coverage for new behavior.

### 8. Single HTTP client: BRRTRouter over may_minihttp

Sesame-IDAM services share a single may coroutine runtime. **No `reqwest`, `tokio::spawn`, or any other async runtime HTTP client is allowed.** Service code must use `sesame_common::http` / `brrtrouter::http`; BRRTRouter uses `may_minihttp::client::HttpClient` for plain HTTP. Background tasks must use `may::task::spawn`.

See `docs/llmwiki/topics/topic-http-client-policy.md` for details and migration tracking.

---

## Commit discipline

- Commits follow Conventional Commits (`feat(scope):`, `fix(scope):`, `docs(scope):`, `chore(scope):`, `refactor(scope):`).
- **Never push** without explicit human authorization.
- **Never use `--no-verify`** or `--no-verify-commit`. Let pre-commit hooks run and fix what they flag.
- **Never commit secrets** (`.env`, credentials, tokens).
- Prefer small, logically-grouped commits with full messages explaining the *why*.

---

## Wiki obligation (task-relevant reading)

**Every session starts with reading [`docs/llmwiki/index.md`](./docs/llmwiki/index.md) and drilling down only to pages relevant to your task.** This is not optional. The wiki accumulates what earlier agents learned; not reading it means you repeat work already done.

Read `index.md`, pick only pages whose title or heading matches your task. Do not load the entire wiki.

If the wiki is out of date or contradicts the code, fix it **in the wiki** as part of your session's deliverable, per [`docs/llmwiki/SCHEMA.md`](./docs/llmwiki/SCHEMA.md) "Agent workflow".

End-of-session: update the wiki pages you touched, append a `log.md` entry, flag any `> **Open:**` questions. Leave the wiki one step more useful than you found it.
