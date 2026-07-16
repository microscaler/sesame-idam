# Sesame-IDAM deployment profiles

Sesame-IDAM owns product configuration under:

```text
deployment-configuration/profiles/<environment>/sesame-idam/<suite>/
```

The IDAM dev profile is `deployment-configuration/profiles/dev/sesame-idam/idam/`.
It has three reconciliation boundaries:

```text
idam/
├── runtime/       # namespace-local ConfigMap and application Secrets
├── bootstrap/     # rerunnable database Job in data
└── services/      # delivered microservice HelmReleases
```

Non-secret settings live in `runtime/application.properties`; secrets are
SOPS-encrypted dotenv files. Bootstrap has a separately encrypted copy of the
product DB password because Kubernetes forbids cross-namespace Secret refs.
Platform administrator credentials are never copied into `sesame-idam`.

Platform configuration remains in `shared-gitops-k8s-cluster`. PostgreSQL HA's
`postgres-ha` profile owns Pgpool's custom-user list — the `sesame_idam`
password there must match `sesame-idam-db-credentials`.

## Ownership

| Owner | Responsibility |
|-------|----------------|
| Flux | SOPS profiles, bootstrap Job, HelmReleases, drift |
| Tilt | Build/push `dev-<nanoseconds>` images; manual migrations |

Encrypt secrets only on ms02:

```bash
export SOPS_AGE_KEY_FILE=~/.config/sops/age/flux-shared-gitops
sops --encrypt --in-place --input-type dotenv --output-type dotenv \
  deployment-configuration/profiles/dev/sesame-idam/idam/runtime/application.secrets.env
```

### JWT signing key (`sesame-idam-jwt-signing`)

Login and session **must** share one Ed25519 key. Without it, login signs with
`kid=dev-ephemeral` while session JWKS publishes a different `key-*` kid — Hauliage
BFF (and any JWKS consumer) returns `401 invalid_token` even when `iss`/`aud` match.

Generate and encrypt (ms02):

```bash
cd microservices/idam/common && cargo run --example print_jwt_signing_env \
  > ../../../../deployment-configuration/profiles/dev/sesame-idam/idam/runtime/jwt-signing.secrets.env
export SOPS_AGE_KEY_FILE=~/.config/sops/age/flux-shared-gitops
sops --encrypt --in-place --input-type dotenv --output-type dotenv \
  deployment-configuration/profiles/dev/sesame-idam/idam/runtime/jwt-signing.secrets.env
```

After Flux reconciles `sesame-idam-idam`, restart login + session:

```bash
kubectl -n sesame-idam rollout restart deploy/identity-login-service deploy/identity-session-service
```

Re-login and confirm token header `kid` matches `kubectl … jwks.json | jq '.keys[].kid'`.

### JWT signing key (`sesame-idam-jwt-signing`)

`identity-login-service` and `identity-session-service` must share one Ed25519 key
(`SESAME_JWT_SIGNING_KEY_PKCS8_B64` + `SESAME_JWT_SIGNING_KID`). Without it, login
signs with `kid: dev-ephemeral` while JWKS publishes a different ephemeral key —
Hauliage BFF (and any JWKS consumer) returns `401 invalid_token` even when `iss`
and `aud` match.

Generate and encrypt (ms02, repo root):

```bash
just jwt-signing-material > deployment-configuration/profiles/dev/sesame-idam/idam/runtime/jwt-signing.secrets.env
export SOPS_AGE_KEY_FILE=~/.config/sops/age/flux-shared-gitops
sops --encrypt --in-place --input-type dotenv --output-type dotenv \
  deployment-configuration/profiles/dev/sesame-idam/idam/runtime/jwt-signing.secrets.env
```

Commit the encrypted file, let Flux reconcile `sesame-idam-idam`, then restart
login + session pods. Re-login; JWT header `kid` must match a key in JWKS.
