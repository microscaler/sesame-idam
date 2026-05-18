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

| Service | Path | Port | Access Pattern | Endpoints |
|---------|------|------|----------------|-----------|
| identity-login-service | `microservices/idam/identity-login-service/` | 8101 | HIGH — login, register, social OAuth, OTP, passwordless | 20 |
| identity-session-service | `microservices/idam/identity-session-service/` | 8105 | HIGH — refresh, OIDC, JWKS, step-up, impersonation, MCP | 13 |
| identity-user-mgmt-service | `microservices/idam/identity-user-mgmt-service/` | 8106 | MEDIUM — user CRUD, MFA, email/phone, passwordless | 25 |
| authz-core | `microservices/idam/authz-core/` | 8102 | EXTREME — every consumer API request | 4 |
| api-keys | `microservices/idam/api-keys/` | 8103 | HIGH — M2M key validation, archiving | 10 |
| org-mgmt | `microservices/idam/org-mgmt/` | 8104 | LOW — org lifecycle, SSO/SCIM, webhooks, SCIM | 34 |

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
just dev-up          # Kind (shared cluster) + Tilt (port 10351)
just dev-down        # Stop Tilt
just supabase-apply  # Apply Supabase stack once (namespace: data)
just port-forward    # Forward postgres (data) + redis (sesame-idam)
just tilt-up         # Start Tilt via systemd
just tilt-log        # Tail Tilt logs
```

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
    2.  **Database layer:** `SesameExecutor` runs `SET LOCAL current_tenant_id = ?` per transaction.
    3.  **RLS policies:** PostgreSQL policies enforce `WHERE tenant_id = current_tenant_id` as a failsafe.

## Core rules the agent must obey

Each rule points at the authoritative source. Open the source when the rule is ambiguous.

### 1. Never edit generated code under `gen/`

`microservices/<service>/gen/` is regenerated by `brrtrouter-gen` from `openapi/<service>/openapi.yaml`. Any edit will be clobbered on next regen. Fix the OpenAPI spec instead.

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
