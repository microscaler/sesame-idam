---
title: Environment Bootstrap Order
status: verified
updated: 2026-07-09
sources: [Tiltfile, justfile, scripts/setup-db.sh, PRD_k8s-native-idam-platform-and-hauliage-integration.md]
---

# Environment Bootstrap Order

Canonical order for standing up sesame-idam on a fresh cluster (shared-k8s on
ms02, or GKE staging). Mirrors the hauliage pattern. PRD FR-DB-05.

## Sequence

1. **Shared-k8s platform** — cluster up, platform namespaces applied
   (`data`, etc.). Postgres and Redis are the shared platform instances in
   namespace `data` (managed by shared-k8s-cluster, `k8s/platform-data/data`):
   `postgres.data.svc.cluster.local:5432` (read replicas `postgres-replica-0`
   / `postgres-replica-1`) and `redis.data.svc.cluster.local:6379`. Sesame
   deploys **no app-local data stack**.
2. **sesame-idam namespace** — `k8s/microservices/namespace.yaml`
   (applied by Tilt; `just dev-up` also ensures it).
   - **JWT signing Secret** — `just dev-up` provisions
     `sesame-idam-jwt-signing` (idempotent; explicit rotation via
     `just jwt-signing-secret`). identity-login-service signs with this key
     and identity-session-service publishes its public half in JWKS — without
     it both fall back to mismatched ephemeral keys and every protected route
     returns 401 (JWT `kid` not in JWKS).
3. **Database env** — `k8s/microservices/database-env.yaml`: ConfigMap
   `sesame-idam-database-config` + Secret `sesame-idam-db-credentials`.
   Applied by Tilt (`sesame-idam-database-env` resource) **before** any Helm
   service deploy (`resource_deps`).
4. **DB init** — Tilt `sesame-idam-db-init` → `scripts/setup-db.sh`:
   idempotently creates role `sesame_idam`, database `sesame_idam`, schema
   `sesame_idam`, then applies `migrations/*.sql` (via
   `migrations/apply_order.txt`) and seeds
   (`microservices/idam/*/impl/seeds/*.sql` via
   `microservices/idam/seed_order.txt`), then grants DML.
   - Re-apply migrations/seeds only: `tilt trigger sesame-idam-apply-migrations`
     (`SESAME_IDAM_APPLY_MIGRATIONS_ONLY=1`).
   - Regenerate migration SQL from Lifeguard models:
     `tilt trigger sesame-idam-migrate`.
5. **Services** — six Helm deploys (ClusterIP :8080), each depending on
   `sesame-idam-database-env`. Values merge order: per-service →
   `_http-kubernetes.yaml` → `_database-kubernetes.yaml`.

## Ports

- In-cluster: every service is ClusterIP **8080 → 8080** (named port `http`);
  binaries read the `PORT` env var (default 8080).
- Host debug (optional, Tilt): login `8101:8080`, session `8105:8080`.
- `just port-forward`: postgres `5432` + redis `6379`, both from the shared
  platform namespace `data`.

## Consumers (hauliage)

Hauliage services call sesame in-cluster at
`http://{service}.sesame-idam.svc.cluster.local:8080/idam/v1/...` — see
hauliage `helm/hauliage-microservice/values/_sesame-idam-kubernetes.yaml`.
Coordinate any port/URL change with hauliage in one release window (PRD
NFR-04).
