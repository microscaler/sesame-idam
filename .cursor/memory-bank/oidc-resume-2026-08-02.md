# OIDC / Epic resume — 2026-08-03

## Shipped

| Item | Note |
|---|---|
| Epic 14–15 | On `main` |
| Epic 16 selection | Rust-only, BFF mandate |
| Client `b32d823` | `SESAME_PUBLIC_API_BASE_URL` / `from_public_api_base` |
| Hauliage seed | `hauliage` tenant + Google OAuth metadata + `hauliage-web` |
| Hauliage config | Public edge `api.` / JWKS on `id.` + caBundle |

## Operator remaining

Set live Google secrets on identity-login-service:

- `SESAME_OAUTH__HAULIAGE__GOOGLE_CLIENT_ID`
- `SESAME_OAUTH__HAULIAGE__GOOGLE_CLIENT_SECRET`

Re-apply platform tenant seed; redeploy Hauliage BFF; E2E Google sign-in.

## Active Epic 16

16.1 done · 16.12 in progress (live secrets/E2E) · 16.11 Rust contract sync · 16.2 template pending
