# Resume — 2026-08-11

## Done — QUERY CORS end-to-end
- BRRTRouter `ae47bfe7` — Askama template + default_cors_allowed_methods
- hauliage `7e910cac` — pin ae47bfe7 + all loadlinker gen/config QUERY
- sesame-idam `c0d9dae` — 6× gen/config QUERY (path dep)

## Done — Series A pre-auth client_id (helper + P0/P1)
- `ClientRegistry::resolve_pre_auth` shared helper
- Migrated: auth_login, auth_register, signup_validate, social_login,
  auth_forgot_password, auth_reset_password
- social_callback: tenant from OAuth state (client_id stored in state at start)
- Client: social_login_start injects client_id, omits X-Tenant-ID
- Verified: login nextest filters + contract_sync + header-free register/login smoke

## Done — Pact HTTP for P0/P1
- `Sesame-Identity-Login-PreAuth.json` + provider verify + client live replay
- Run: `SESAME_PACT_PROVIDER_BASE=http://127.0.0.1:18081/idam/v1 cargo nextest run … pact_preauth`

## Still open
- Series A later: email/phone OTP, magic link, auth_token auth-code path, auth_session_code
- Dual OTP stubs → Series B
- Series B CS transfer / MFA
- Flux/registry digest mismatch blocking login image push (pact verified via local binary)
