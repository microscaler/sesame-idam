# sesame-idam-microservice

Helm chart for the six Sesame-IDAM microservices. Same pattern as hauliage
`hauliage-microservice`.

## Services

| Service | Values file |
|---------|-------------|
| identity-login-service | `values/identity-login-service.yaml` |
| identity-session-service | `values/identity-session-service.yaml` |
| identity-user-mgmt-service | `values/identity-user-mgmt-service.yaml` |
| authz-core | `values/authz-core.yaml` |
| api-keys | `values/api-keys.yaml` |
| org-mgmt | `values/org-mgmt.yaml` |

## Port convention (breaking change)

> **BREAKING:** All services previously used Kind-era unique host ports
> **8101–8106** with `serviceType: NodePort`. As of the 8080/8080 migration
> (PRD: `docs/PRD_k8s-native-idam-platform-and-hauliage-integration.md`) every
> Service is **ClusterIP** with **port 8080 → targetPort 8080** and a named
> `http` port. Service identity is the Kubernetes Service name, not a port
> number, e.g.
> `http://identity-login-service.sesame-idam.svc.cluster.local:8080/idam/v1`.
> There are no `nodePort` values; debugging uses `kubectl port-forward` (Tilt
> optionally forwards login at `8101:8080` and session at `8105:8080` on the
> host for isolated Sesame debugging).

The deployment sets the `PORT` env var from `service.containerPort`; all impl
binaries honour `PORT` and default to 8080.

## Shared values overlays

Tilt merges values in this order (later files override earlier):

1. `values/_dev.yaml` — `devMode: true` and the development issuer and public URLs
2. `values/{service}.yaml` — per-service identity, image, authz wiring
3. `values/_http-kubernetes.yaml` — cluster-wide 8080/ClusterIP
4. `values/_database-kubernetes.yaml` — `app.config.database.*` +
   `app.config.redis.*` pointing at the shared platform data stack in
   namespace `data`: `postgres.data.svc.cluster.local` /
   `redis.data.svc.cluster.local`

Database credentials come from the `sesame-idam-db-credentials` Secret
(`k8s/microservices/database-env.yaml`); the values-file password is a dev
fallback only.

## Deployment settings (no defaults)

The chart defaults are not a development environment. Outside `devMode` it refuses to render a service without:

- `env.SESAME_JWT_ISSUER` (every service), and for identity-login-service and identity-session-service also
  `env.SESAME_AUTH_PUBLIC_URL`, `env.SESAME_API_PUBLIC_URL`, `signingKeyset.enabled=true` (ADR-006) and
  `idTokenKeys.configMap` (ADR-015);
- the services repeat the check at start-up (`sesame_common::deployment`): an https URL on the deployment's own domain
  (no `.local`, `example.com`, `localhost` or IP), and the keys mounted.

`devMode: true` sets `SESAME_DEV_MODE=1`, which turns these into warnings and allows the development fallbacks
(including an ephemeral signing key). Only `values/_dev.yaml` and the dev profile set it.

## Manual deploy

```bash
helm upgrade --install identity-login-service ./helm/sesame-idam-microservice \
  -f ./helm/sesame-idam-microservice/values/_dev.yaml \
  -f ./helm/sesame-idam-microservice/values/identity-login-service.yaml \
  -f ./helm/sesame-idam-microservice/values/_http-kubernetes.yaml \
  -f ./helm/sesame-idam-microservice/values/_database-kubernetes.yaml \
  -n sesame-idam
```

Repeat per service. Normally Tilt (`just dev-up`) does this for all six.
