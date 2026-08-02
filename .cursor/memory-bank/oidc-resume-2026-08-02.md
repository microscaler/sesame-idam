# OIDC resume — 2026-08-03

## Done
### Pushed
- `5305e39` interactive PKCE + Epic 14 runner + pre-OIDC gates
- `ed7f4ff` BearerAuth `iss` aligned with `SESAME_JWT_ISSUER` (+ helm
  ConfigMap force + `jwks_issuer_alignment` drift test)

### Live verified (ms02 / shared-k8s)
Full interactive PKCE green:
`authorize → login → authorize/complete → token → userinfo`

- JWT trust: ConfigMap `iss=https://id.sesameidentity.dev.local`
- Login image includes authorize `tenant`/`client_id` query + UserInfo RLS
- Hosted auth SPA rebuilt with `completeOidcAuthorize` (`index-CecitWoN.js`)
- `oidc_` BDD: **24/24 passed** including `live_interactive_pkce_round_trip`

### Secrets (hauliage-aligned)
Passwords/keys only via SOPS `*.secrets.env` / `*.secret.yaml` + kustomize
`secretGenerator`. Never put plaintext credentials in helm values.

## Pending / follow-ups
1. Commit remaining: remove live soft-skip; pin `frontend-auth` image tag +
   registry repository in Flux HelmRelease (no ImageRepository for frontends yet)
2. Add Flux ImageRepository/ImagePolicy for `sesame-idam-frontend-*` (or keep
   manual dig tags)
3. Epic 14.6–14.7 external conformance / framework matrix
4. Dual OTP / SMS verify still unwired stubs

## Ops notes
- Sesame Tilt UI: `http://ms02:10351`
- Force login binary publish: tilt trigger
  `build-` → `copy-` → `image-sesame-idam-identity-login-service`, then
  `flux reconcile image repository/update` + HR
- Auth frontend: `docker build -f docker/frontend/Dockerfile --build-arg APP=auth`
  push `dev-<ns>`; HR currently pins registry + dig tag
