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
