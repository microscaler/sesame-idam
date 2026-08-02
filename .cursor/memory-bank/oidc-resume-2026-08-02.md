# OIDC / pre-OIDC resume — 2026-08-03

## Pushed
- `5305e39` feat(oidc): interactive PKCE path, Epic 14 runner, and pre-OIDC live gates

## In progress (uncommitted): JWT iss drift fix
Root cause of live `authorize/complete` `invalid_token`: minted tokens use
`SESAME_JWT_ISSUER=https://id.sesameidentity.dev.local` but ConfigMap
`security.jwks.BearerAuth.iss` was still `https://idam.example.com`.

Pending git changes (NO secrets — public issuer URLs only):
- helm values `*.yaml` BearerAuth.iss → id.sesameidentity.dev.local
- Flux `common.yaml` + login/session HelmRelease iss
- configmap template forces iss from `env.SESAME_JWT_ISSUER`
- `common/tests/jwks_issuer_alignment.rs` drift guard

Live hot-patched ConfigMaps + restarted services for verification.
Complete JWT path unblocked; userinfo/authorize follow-ups still open.

## Secrets policy (hauliage-aligned)
Sesame already matches hauliage SOPS layout — do **not** put plaintext
passwords/keys in helm values or git:

| Pattern | Path |
|---------|------|
| `.sops.yaml` | repo root — `*.secrets.env` + `*.secret.yaml` age rules |
| DB bootstrap | `…/bootstrap/application.secrets.env` → secretGenerator `sesame-idam-bootstrap-db` |
| DB runtime | `…/runtime/application.secrets.env` → `sesame-idam-db-credentials` |
| JWT signing | `…/runtime/jwt-signing.secrets.env` + `signing-keyset.secret.yaml` |
| Twilio | `…/runtime/twilio.secrets.env` |

Helm `app.config.database.password: ""` — real password via Secret/`DB_PASS` envFrom.
Decrypt: `SOPS_AGE_KEY_FILE=~/.config/sops/age/…` (see docs/sops-age-keys.md).

## Next
1. Commit/push iss alignment (public URLs only)
2. Finish live interactive (userinfo aud if still 401)
3. Redeploy auth frontend for SPA OIDC path
