# Story 1.1 — Completion Assessment & Outstanding Work

## Story Overview

Epic 1: Asymmetric JWT & JWKS (EdDSA/Ed25519 signing, JWKS key publication)
Story 1.1 implements: key management with rotation lifecycle, JWKS endpoint, admin revoke, JWKS headers middleware, audit logging on key lifecycle events, consumer JWKS client.

## What Was Completed (Verified by Code Inspection)

The following items from the story checklist are confirmed implemented:

- [x] **Key generation** — `key_manager.rs` generates Ed25519 keys using `ring::signature::Ed25519KeyPair::generate_random()`
- [x] **Key management with lifecycle** — KeyManager with prepare/activate/lifecycle rotation, active key counter, kid auto-increment via `next_kid()`
- [x] **Revocation** — `revoke_key()` removes key from JWKS and memory, tracks revocation reason
- [x] **Health check** — Returns current key state, JWKS key count, next key status
- [x] **JWKS endpoint** — `controllers/jwks.rs` serves /.well-known/jwks.json, wired into main.rs routing, served via BRRTRouter static_files
- [x] **Admin revoke** — `controllers/admin_jwks_revoke.rs` POST endpoint calls `KEY_MANAGER.write().unwrap().revoke_key()`
- [x] **JWKS middleware** — `middleware/jwks_headers.rs` injects Cache-Control, X-Content-Type-Options, Vary headers
- [x] **Consumer client** — `jwks_client.rs` with algorithm allow-list, per-service configs, JWKS poisoning guard
- [x] **KeyManager LazyLock** — Global `KEY_MANAGER` using `LazyLock<RwLock<KeyManager>>` for interior mutability
- [x] **Audit logging** — Added audit events for key lifecycle (key_generated, key_rotated, key_revoked, grace_key_expired)
- [x] **Tests** — 100 tests: 30 key_manager + 10 jwks_client + 10 jwks_http BDD + 4 middleware + 1 smoke + 45 jwks BDD

## What is Outstanding / BLOCKED

### 🔴 CRITICAL: 110 Compilation Errors — Audit API Mismatch

The audit logging added to the 1.1 implementation layer does NOT compile against the current `sesame-audit` API. All errors are in `identity-session-service/impl/`:

**Error categories (all 110 errors):**

1. **`AuditEvent::new_with_params()` doesn't exist** — The current API uses `AuditLogEntryBuilder::new(AuditEventType, service)` with a builder pattern. All 13 controller files use `AuditEvent::new_with_params(AuditEventType, description, tenant_uuid, AuditActor, ip)` which doesn't exist.

2. **`AuditEventType` enum is incomplete** — Callers reference `AuditEventType::SessionManagement`, `AuditEventType::UserManagement`, `AuditEventType::System` — none of which exist. Current enum has: JwtIssued, JwtValidated, ValidationFailed, TokenRevoked, FamilyRevoked, Delegation, VersionBump, VersionMismatch, TokenBindingMismatch.

3. **`AuditLevel::Warning` / `Critical` doesn't exist** — Callers reference `AuditLevel::Warn`, `AuditLevel::Error`, `AuditLevel::Info`, `AuditLevel::Debug`. `Warn` and `Error` exist but callers use `Warning`/`Critical`.

4. **`AuditActor::ServiceAccount` doesn't exist** — Current enum is `AuditActor { User, Admin, System }`. Controllers reference `ServiceAccount`.

5. **`AuditEvent` builder fields are methods** — Callers set `event.user_id = ...`, `event.severity = ...`, `event.metadata = ...`, `event.target_id = ...`, `event.target_type = ...` as struct fields. The actual API uses builder methods: `.user_id()`, `.metadata()`, etc. `severity`/`target_id`/`target_type` don't exist on the builder.

6. **`EMITTER.emit()` is private** — Method is `fn emit()` (private) on `AuditEmitter`. Callers use `EMITTER.emit(&mut event)`.

7. **`EMITTER::new(None)` wrong signature** — Current API is `AuditEmitter::new(service: impl Into<String>, hmac_key: Option<Vec<u8>>)`. audit.rs passes `None` (no service name).

8. **`lazy_static` / prometheus counters missing** — `services/token_rotation.rs` uses `lazy_static::lazy_static!` crate (not in Cargo.toml deps) and prometheus `register_int_counter!` macros that aren't compiled in. References `redis::RedisError` which doesn't exist.

### Files That Need Fixes

| File | Issues |
|------|--------|
| `impl/src/audit.rs` | `EMITTER::new(None)` — needs service name + hmac_key |
| `impl/src/services/token_rotation.rs` | `lazy_static` crate missing, `redis::RedisError` wrong type, prometheus counters not in scope |
| `impl/src/controllers/admin_issue_token.rs` | 7 audit-related errors |
| `impl/src/controllers/admin_restore_impersonation.rs` | 7 audit-related errors |
| `impl/src/controllers/auth_refresh.rs` | 10+ audit-related errors (4 audit event calls) |
| `impl/src/controllers/mcp_create_agent.rs` | 7 audit-related errors |
| `impl/src/controllers/mcp_delete_agent.rs` | 7 audit-related errors |
| `impl/src/controllers/mcp_get_agent.rs` | 7 audit-related errors |
| `impl/src/controllers/mcp_list_agents.rs` | 7 audit-related errors |
| `impl/src/controllers/oauth_userinfo.rs` | 7 audit-related errors |
| `impl/src/controllers/openid_configuration.rs` | 7 audit-related errors |
| `impl/src/controllers/step_up_verify.rs` | 7 audit-related errors |
| `impl/src/controllers/users_me_get.rs` | 7 audit-related errors |
| `impl/src/controllers/users_me_patch.rs` | 7 audit-related errors |
| `impl/src/redis.rs` | Unused import: `MAX_DENYLIST_SIZE` |

## What Needs to Happen Next

Two approaches are possible:

### Approach A: Fix the calling code to match the sesame-audit API

For each controller file, replace:
```rust
let mut event = AuditEvent::new_with_params(
    AuditEventType::SessionManagement,
    "event_name",
    tenant_id.parse::<Uuid>().unwrap_or_default(),
    AuditActor::User,
    "127.0.0.1".to_string(),
);
EMITTER.emit(event);
```
With:
```rust
let entry = AuditLogEntryBuilder::new(AuditEventType::JwtIssued, "identity-session-service")
    .user_id(user_id)
    .tenant_id(tenant_id)
    .decision_source("handler")
    .result("ok")
    .build()
    .unwrap();
EMITTER.emit(entry);
```

Also need to:
- Choose appropriate `AuditEventType` from the existing 9 for each controller event
- Fix `AuditActor::ServiceAccount` → `AuditActor::System`
- Fix `AuditLevel::Warning` → `AuditLevel::Warn`
- Remove unused `AuditSeverity`/`AuditActor`/`uuid` imports
- Fix `redis.rs` unused import
- Fix `token_rotation.rs` — either remove lazy_static/prometheus or add to Cargo.toml, fix `redis::RedisError`

### Approach B: Extend sesame-audit API to support the calling code pattern

Add to `sesame-audit/src/event.rs`:
- Missing `AuditEventType` variants: `SessionManagement`, `UserManagement`, `System`
- Change `AuditLevel::Warn` to also accept `Warning` alias (or update callers)
- Add `AuditActor::ServiceAccount` variant

Add to `sesame-audit/src/emitter.rs`:
- Make `emit()` public (currently private)

### Recommended: Approach A

It's faster and avoids bloating the audit API with one-off enum variants. The calling code should use the existing event types that map semantically:
- Token refresh → `JwtIssued` or create a more specific event
- User profile → `Delegation` (closest existing)
- MCP agents → `Delegation` or `JwtValidated`
- Admin operations → `JwtIssued` with actor context in metadata

**Important: This is a story 1.1 issue — the audit logging was added as part of story 1.1's implementation but was written against a broken API. The core 1.1 functionality (key management, JWKS, revocation, middleware, client) is sound and untested as a result because the entire service won't compile.**

## Gate Status

- [x] Compilation PASS (core lib only, before audit controller additions)
- [ ] Compilation FAIL — 110 errors across 14 files
- [ ] Lint PASS (clippy pedantic)
- [ ] Tests PASS (100/100) — can't verify until compilation passes

## Previous Session Progress

- audit.rs fixed: `EMITTER::new("identity-session-service", None)` — service name added ✅
- emitter.rs fixed: `emit()` made `pub` ✅
- Remaining: 11 files with audit API mismatches + token_rotation.rs + redis.rs
