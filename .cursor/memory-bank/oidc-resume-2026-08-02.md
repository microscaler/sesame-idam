# OIDC / pre-OIDC resume — 2026-08-02 (late)

## OIDC next wave (this turn)
### Shipped
- Hosted auth SPA: OIDC path when `request_id` present → `POST /oauth/authorize/complete`
  (`frontend/auth` App.tsx + api.ts; login sends `client_id`)
- Authorize redirect now includes `tenant` + `client_id` for hosted auth
- Fixture public clients seed: `20260802220000_oidc_fixture_public_clients.sql`
  (apply with `set_config('app.tenant_id','hauliage',true)`)
- In-process interactive PKCE: `bdd/oidc_interactive.rs` — authorize→login→complete→token→userinfo **green**
- Epic 14 runner: `bdd/oidc_conformance.rs` — manifest + redirect-prefix/PKCE-plain/valid PKCE/code replay **green**
- UserInfo RLS fix: `with_pre_auth_tenant` for subject lookup
- Live interactive: authorize+login work; **complete skipped** when edge returns `invalid_token` on login access JWT

### Verified on ms02
- `oidc_interactive` ok
- `oidc_conformance` 5/5 ok
- `oidc_` suite: live_interactive soft-skips on complete JWT; rest green

### Follow-ups (still open)
1. **JWT trust**: login-issued access tokens rejected by BRRTRouter on `/oauth/authorize/complete` (`WWW-Authenticate: Bearer error="invalid_token"`) — same via ClusterIP. Fix validator/JWKS/audience so live interactive can finish.
2. Redeploy login + auth frontend images so hosted UI + authorize query enrichment are live.
3. External OpenID conformance suite / framework matrix (Epic 14.6–14.7)
4. Refresh rotation fixtures still stubbed in protocol-cases (not wired to session refresh)

## Pre-OIDC gate (previous) — still required green before claiming OIDC done
auth_flow, north_live, east_west, social, sms_magic, stubs, session_handoff, token_lifecycle

## Apply fixture seed
```bash
PGPASSWORD=… psql -h 10.177.76.224 -U sesame_idam -d sesame_idam <<EOF
BEGIN;
SELECT set_config('app.tenant_id', 'hauliage', true);
\i microservices/idam/identity-login-service/impl/seeds/20260802220000_oidc_fixture_public_clients.sql
COMMIT;
EOF
```
