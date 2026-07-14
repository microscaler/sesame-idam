# Sesame-IDAM — Contributing Guide

## The Golden Rule

**No story is complete until it passes every gate.** There is no partial credit. A story with a gap in any single gate is **not done**.

If a story is missing even one item — compile, pedantic lint, unit tests, BDD E2E tests — it stays marked as Incomplete.

## Completion Gates (All Must Pass)

### Gate 1: Compilation

```
cargo check --workspace
```

The entire workspace must compile. No new warnings, no new errors. If a story introduces a new crate dependency, update `Cargo.toml` and `Cargo.lock`.

### Gate 2: Pedantic Linting

```
just lint-rust
```

Runs clippy with `-D warnings -W clippy::pedantic`. Pedantic mode is **mandatory for security-critical code**. Numeric thresholds are in `clippy.toml`:

- `stack-size-threshold = 512000`
- `cognitive-complexity-threshold = 30`
- `too-many-arguments-threshold = 8`

No new warnings, no pedantic violations. If you suppress a pedantic lint, document why in a `#[allow(...)]` comment on the same line and reference the issue/reason.

### Gate 3: Unit Tests

All new and modified code must have unit tests. Every function that has non-trivial logic needs at least one `#[test]`. Use the `#[cfg(test)]` module at the bottom of the source file or in a sibling `*_tests.rs` file.

```
cargo nextest run -p <crate-name>
```

All tests must pass. New failures = the feature broke something.

### Gate 4: BDD E2E Spec Tests

Every story has acceptance criteria with BDD-style scenarios in its `docs/Epics/{N}-{name}/stories/story-N.M.md` file. These are not optional.

```
just nt
```

All 48+ workspace tests must compile and run clean. BDD integration tests use `rstest_bdd` with the five-section pattern:

1. **Unit**: mock I/O, logic correctness
2. **BDD Integration**: given/when/then with actual endpoints
3. **Security Regression**: tenant isolation, token tamper, privilege escalation
4. **Edge Cases**: malformed input, concurrency, empty fields, max limits
5. **Cleanup**: Redis FLUSHDB/prefix, metrics reset, mock server

## Story Completion Checklist

Before marking any story as complete, verify ALL of:

- [ ] `cargo check --workspace` — 0 errors, 0 new warnings
- [ ] `just lint-rust` — clippy pedantic clean
- [ ] `cargo nextest run` — all tests pass (new + existing)
- [ ] BDD E2E acceptance criteria scenarios pass
- [ ] OpenAPI spec matches the impl (no drift)
- [ ] Wiki pages updated per story's "Wiki Pages to Update/Create" section
- [ ] DESIGN DOC CHANGES: `design-doc.md`, `sesame-idam-complete.md`, `service-topology-design.md` updated per epic's "Design Doc Changes Required" section
- [ ] `log.md` entry appended to `docs/llmwiki/log.md`

If ANY gate fails, the story is not complete. Do not mark it done.

## Pre-commit Hooks

```
just install-hooks
```

Run `just qa` before committing. It executes the Python-side lint + format checks. Rust-side gates (clippy, nextest) run via pre-commit hooks.

## Platform tenant administration

Sesame-IDAM is **SaaS-of-SaaS**: each customer product is a **platform tenant** whose slug matches `X-Tenant-ID`. Tenants must be **provisioned** before any auth endpoint accepts traffic. Use the platform admin REST API or the `sesame-idam tenant` CLI — do not hand-edit tenant rows for routine onboarding.

**Tenant-agnostic rule:** Use generic example slugs in docs, commits, and public examples (e.g. `acme-corp`). Do not embed names of internal dogfood tenants in the public repo.

### Prerequisites

- `identity-login-service` reachable (local port-forward or in-cluster)
- Platform admin key configured (`X-Platform-Admin-Key`)

### Environment

| Variable | Purpose | Local dev default |
|----------|---------|-------------------|
| `SESAME_LOGIN_SERVICE_URL` | Login-service base URL (includes `/idam/v1`) | `http://127.0.0.1:8101/idam/v1` |
| `SESAME_PLATFORM_ADMIN_KEY` | Value sent as `X-Platform-Admin-Key` | `dev-platform-admin` (see `identity-login-service/impl/config/config.yaml`) |

### CLI setup

```bash
just init   # once: tooling/.venv + sesame-idam CLI
```

The `sesame-idam` CLI delegates workspace commands (codegen, lint, etc.) to BRRTRouter. The `tenant` subcommands call the platform REST API over HTTP — **not** the database.

> **Note:** If `tooling/.venv` was created on ms02 (Linux), run `just qa` and `sesame-idam tenant …` on ms02. NFS-mounted venvs are architecture-specific.

### Quick usage

```bash
export SESAME_PLATFORM_ADMIN_KEY="${SESAME_PLATFORM_ADMIN_KEY:-dev-platform-admin}"
export SESAME_LOGIN_SERVICE_URL="${SESAME_LOGIN_SERVICE_URL:-http://127.0.0.1:8101/idam/v1}"

# Mint tenant (active by default; use --no-activate for provisioning state)
sesame-idam tenant create --slug acme-corp --display-name "Acme Corp"

# Inspect tenant + OAuth provider metadata
sesame-idam tenant get --slug acme-corp

# Lifecycle transitions
sesame-idam tenant status set --slug acme-corp --status suspended

# OAuth metadata (secret *values* live in K8s/env; DB stores env key names only)
sesame-idam tenant oauth set --slug acme-corp --provider google \
  --client-id "$GOOGLE_CLIENT_ID" \
  --redirect-uris "https://app.example.com/oauth/callback" \
  --secret-env-key SESAME_OAUTH__ACME_CORP__GOOGLE_CLIENT_SECRET \
  --client-id-env-key SESAME_OAUTH__ACME_CORP__GOOGLE_CLIENT_ID

# Record rotation after updating the secret out-of-band (bumps config_version)
sesame-idam tenant oauth rotate --slug acme-corp --provider google --by ops@example.com
```

### Behaviour

| Situation | Public auth routes | Platform API |
|-----------|-------------------|--------------|
| Unknown slug | `404 tenant_unknown` | `404 tenant_not_found` |
| Non-active tenant (`provisioning`, `suspended`, …) | `403 tenant_not_active` | Allowed (admin can manage) |
| Missing/invalid platform key | — | `401 unauthorized` |
| CLI/API error | — | Non-zero exit; JSON error body printed |

After `oauth set`, apply the actual OAuth client secret to the cluster (Helm/Tilt/env) using the `secret_env_key` you configured. The API never returns secret values.

### References

- OpenAPI: `openapi/idam/identity-login-service/openapi.yaml` (`PlatformAdmin` tag, `PlatformServiceAuth`)
- Epic: `docs/Epics/10-platform-tenancy/`
- Design: `docs/design-saas-of-saas-multi-tenancy.md`
- Wiki: `docs/llmwiki/topics/topic-platform-tenants.md`

## Workflow for Story Implementation

1. Read the story file in `docs/Epics/{N}-{name}/stories/story-N.M.md`
2. Read the relevant impl models in `microservices/<service>/impl/src/models/*.rs`
3. Implement against the OpenAPI spec in `openapi/<service>/openapi.yaml`
4. Write unit tests alongside impl code
5. Write BDD E2E tests per the story's acceptance criteria
6. Run all four gates above
7. Update wiki pages, design docs, and `log.md`
8. Update `docs/Epics/INDEX.md` implementation status table
9. Commit with Conventional Commits format

## No Exceptions

There is no shortcut around the gates. No `--no-verify`, no `just lint --ignore-pedantic`, no "we'll fix it later." Every gate is mandatory.
