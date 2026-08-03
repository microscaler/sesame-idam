# Epic 16.12 evidence — Hauliage Google dogfood (2026-08-03)

## Decision

Rust-only Supported client + BFF pattern. Hauliage is the first product on the
public Sesame contract with email/password and Google via Sesame social.

## Sesame changes

| Item | Detail |
|---|---|
| Tenant seed | `hauliage` active platform tenant |
| RP client | `hauliage-web` (confidential, portal `frontend`) |
| Google OAuth metadata | `tenant_oauth_providers` for `hauliage`/`google` |
| Redirect allowlist | `https://hauliage.dev.microscaler.local/oauth/callback`, `http://localhost:7174/oauth/callback`, `http://127.0.0.1:7174/oauth/callback` |
| Secret env keys | `SESAME_OAUTH__HAULIAGE__GOOGLE_CLIENT_ID`, `SESAME_OAUTH__HAULIAGE__GOOGLE_CLIENT_SECRET` |

Seed file: `microservices/idam/identity-login-service/impl/seeds/20260714000000_platform_tenants.sql`

## Hauliage changes

| Item | Detail |
|---|---|
| BFF Sesame bases | `https://api.sesameidentity.dev.local/idam/v1` (public edge) |
| JWKS / iss | `https://id.sesameidentity.dev.local` |
| Client | bump `sesame-idam-client` to include `SESAME_PUBLIC_API_BASE_URL` / `from_public_api_base` |
| FE social | existing BFF proxy (`/api/v1/identity/auth/social/google/*`) — unchanged |

## Operator steps (live Google)

1. Re-apply Sesame platform tenant seed (Tilt/migrate path used in env).
2. Create a Google Cloud OAuth **Web** client; authorized redirect URIs must
   exactly match the Hauliage SPA callback(s) above (Google → browser, not Sesame).
3. Set on identity-login-service pods:

   ```text
   SESAME_OAUTH__HAULIAGE__GOOGLE_CLIENT_ID=...
   SESAME_OAUTH__HAULIAGE__GOOGLE_CLIENT_SECRET=...
   ```

4. Redeploy Hauliage BFF with public-edge config + edge CA bundle.
5. Sign in at `https://hauliage.dev.microscaler.local` → Google → BFF callback → Sesame tokens.

## Still deferred

- Moving browser tokens out of `localStorage`
- Hauliage as full OIDC RP (Auth Code + PKCE to `auth.`) — BFF broker remains
- Microsoft social for Hauliage
- Multi-language SDKs

## Compatibility CI (16.11 Rust-scoped)

- `sesame-idam-client` `tests/contract_sync.rs` (tenant-consumer + profile versions)
- Sesame `oidc-conformance-gate` + `contract_sync` (provider side)
