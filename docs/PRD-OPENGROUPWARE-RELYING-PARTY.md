# PRD: OpenGroupware Relying-Party Readiness — OIDC Completion, Protocol Credentials & Trusted-RP Directory Bridge

**Date:** 2026-07-16
**Status:** Draft — ready for implementation
**Phase:** Cross-cutting (hardens P1 surface; precedes opengroupware MVP slice 1)
**Authors:** OpenGroupware (owner: Claude, on behalf of Charles) — boy-scout rule: opengroupware builds what it needs in sesame, as general SaaS-IDAM features
**Design source:** [opengroupware ADR-0006 v2](../../opengroupware/docs/adr/0006-identity-architecture.md), [cross-repo-auth-analysis.md](./cross-repo-auth-analysis.md)
**Depends on:** tenant registry + auth gate (implemented), argon2id password service (implemented), RFC 8693 token-exchange module (implemented)
**Blocks:** opengroupware MVP slice 1 (tenant/account provisioning), opengroupware web SSO, Stalwart IMAP/SMTP/DAV auth

---

## 1. Executive Summary

OpenGroupware (mail/groupware over Stalwart) has adopted sesame-idam as its **only** identity platform (no Authentik, no LLDAP, no product-local credentials). Assessment on 2026-07-16 found four gaps between sesame today and what any *protocol-serving* relying party needs. None are mail-specific — each is a general SaaS-IDAM capability that PropelAuth-class products ship, and each will be exercised (dogfooded) hard by a mail platform:

1. **OIDC is a surface, not a service.** JWTs carry `placeholder_signature` (`auth_token.rs:819,842`); no real RS256 signing, no working JWKS, auth-code issuance incomplete, `client_credentials` returns not-implemented (`auth_token.rs:967`), no RFC 7662 introspection.
2. **No per-user protocol credentials ("app passwords").** Legacy protocol clients (IMAP/SMTP/DAV, but equally: SFTP, MQTT, any non-browser client of a sesame tenant) cannot do OIDC redirects. Users with MFA need scoped, revocable static credentials.
3. **No credential-verification path for trusted first-party protocol servers.** Stalwart authenticates against internal/LDAP/**SQL**/OIDC directories. Sesame offers none of these externally. The cheapest, most general bridge: a read-only, RLS-guarded **relying-party directory schema** in Postgres that trusted RPs may query.
4. **`sesame-idam-client` does not verify tokens** (`claims.rs:51-56` trusts edge validation). Any consumer outside BRRTRouter's edge (e.g. opengroupware's tokio/axum services) has no way to validate a sesame JWT.

Also: two compile-blocking defects found during assessment must be fixed first (§7).

## 2. Problem Statement

| Today | Problem |
|-------|---------|
| `placeholder_signature` on issued JWTs | No RP can trust a sesame token; blocks all SSO |
| No JWKS/introspection | RPs cannot validate tokens offline or online |
| `client_credentials` unimplemented | opengroupware admin-api has no M2M path to call provisioning APIs |
| No app-password concept (`api_keys` is M2M-only) | Protocol clients of any tenant product cannot authenticate; MFA rollout would break them entirely |
| No LDAP/SQL/OIDC directory exposure | Stalwart (and any future protocol server) cannot verify user credentials against sesame |
| Client SDK trusts pre-validated claims | Non-BRRTRouter services must hand-roll JWT verification |
| `auth_token.rs:709` `.await` in sync fn; `build_access_token` arity mismatch | Tree likely does not compile; blocks everything above |

### Why now

- opengroupware MVP slice 1 (tenant/domain/account provisioning) starts immediately and calls sesame APIs with `client_credentials`.
- Other sesame agents are paused; opengroupware drives sesame changes under the boy-scout rule and feeds edge cases back.
- Every feature here generalizes: PriceWhisperer/Hauliage gain real OIDC + M2M; any future protocol product gains app passwords + the RP directory.

## 3. Goals

| # | Goal | Evidence |
|---|------|----------|
| G1 | Issued JWTs are RS256-signed with rotating keys, verifiable via `/.well-known/jwks.json` | Third-party JWT lib validates an issued token end-to-end |
| G2 | Auth-code + PKCE flow works for a registered web client | opengroupware webmail completes login round-trip in dev |
| G3 | `client_credentials` grant issues scoped M2M tokens | opengroupware admin-api provisions a user with it |
| G4 | RFC 7662 introspection endpoint live | Introspecting a revoked token returns `active: false` |
| G5 | App passwords: issue/list/revoke per user, argon2id-hashed, scoped, tenant-isolated | BDD suite; RLS zero-bleed test extended to `app_passwords` |
| G6 | RP directory schema consumable by Stalwart SQL directory | Stalwart in kind authenticates an IMAP login against sesame Postgres |
| G7 | `sesame-idam-client` verifies JWTs via cached JWKS (feature-gated, runtime-agnostic) | opengroupware service-kit validates tokens with it |
| G8 | Workspace compiles clean; placeholder-signature code deleted | CI green; grep for `placeholder_signature` returns nothing |

## 4. Feature: OIDC completion (F1)

- RS256 signing keys: generated per environment, stored via existing secrets pattern, `kid`-versioned, rotation supported (two active keys max, JWKS serves both).
- `/.well-known/openid-configuration` and `/.well-known/jwks.json` return real data (session-service already declares the routes).
- Auth-code + PKCE: complete issuance/redemption against the existing session model; codes single-use, 60s TTL, Redis-backed.
- `client_credentials`: scoped to registered M2M clients (existing `api_keys` table extended with `allowed_scopes`); tokens carry `tenant_id` claims consistent with RLS session contract.
- RFC 7662 `/oauth/introspect`: authenticated by client credentials; respects token versioning (`ver`/`sid`).

## 5. Feature: App passwords / protocol credentials (F2)

New table `app_passwords` (per-user, tenant-scoped, RLS): `id, tenant_id, user_id, label, secret_phc (argon2id), scopes text[], created_at, last_used_at, revoked_at`. Endpoints on user-mgmt service: create (returns plaintext exactly once), list, revoke. Constraints: max 25 per user; generation server-side only (no user-chosen secrets); `last_used_at` updated at most once per 5 min to avoid write amplification. Not JWTs, not sessions — verified only via F3 or a dedicated verify endpoint.

## 6. Feature: Trusted-RP directory bridge (F3)

A dedicated Postgres schema `rp_directory` with **read-only views**, accessed by a dedicated DB role per relying party (e.g. `rp_stalwart`), locked to `SELECT`:

- `rp_directory.users(tenant_slug, login, secret_phc, display_name, quota_bytes, active)`
- `rp_directory.app_passwords(tenant_slug, login, secret_phc, scopes, active)`
- `rp_directory.groups` / `memberships` (Stalwart mailing-list expansion later)

Rationale: Stalwart's SQL directory verifies standard PHC hashes natively; this avoids building an LDAP server (explicitly out of scope, per SEASAME_ANALYSIS gap list) while remaining product-agnostic — any trusted first-party protocol server gets the same bridge. Isolation: views filter `active` tenants only; RP roles are per-consumer and revocable; connection via pgbouncer with statement-level auditing enabled.

## 7. Fixes & client SDK (F4/F5)

- Fix `auth_token.rs:709` (`.await` inside sync fn) and the 8-vs-9 arg `build_access_token` call; delete placeholder-signature paths (G8).
- `sesame-idam-client`: add `verify` module — JWKS fetch + cache (TTL 10 min), RS256 validation, `ValidatedClaims` output identical to today's struct so BRRTRouter consumers are unaffected. Feature-gated `verify-tokio` / `verify-may` to respect both runtimes; no default runtime dependency.

## 8. Out of scope

LDAP server, SCIM, SAML assertion processing, WebAuthn (tracked separately), TOTP completion (separate PRD — placeholder stub noted), self-service store (P3).

## 9. Risks

| Risk | Mitigation |
|------|------------|
| RP directory exposes PHC hashes to a compromised RP | Per-RP roles, SELECT-only, active-tenant filter, hash-only (argon2id is offline-attack resistant), audit log on RP connections |
| Key rotation breaks live RPs | Two-key JWKS overlap window ≥ 24h; `kid` mandatory |
| Runtime split (may vs tokio) in client SDK | Feature gates; verification logic pure-Rust, IO abstracted |
| Mail MVP schedule coupling | F1/F3 are the critical path; F2 can land one sprint later (Stalwart falls back to primary password until then) |

## 10. Rollout

1. F5 compile fixes (unblocks CI) → 2. F1 signing/JWKS → 3. F3 schema + Stalwart kind smoke test → 4. F1 auth-code/M2M/introspection → 5. F4 client verify → 6. F2 app passwords. Each step lands with BDD coverage per repo convention; RLS zero-bleed suite extended for every new table/view.
