# Sesame-IDAM data components

Sesame-IDAM runs **no app-local data stack**. Postgres and Redis are the shared
platform instances in namespace `data`:

- **Postgres HA (Bitnami / Flux `stack-postgres-ha`):**
  - App endpoint: `postgres.data.svc.cluster.local:5432` (Pgpool LB alias)
  - Privileged bootstrap: elected primary in StatefulSet `postgres-ha-postgresql`
  - Database `sesame_idam` is bootstrapped by Flux Job `scripts/db-init-job.sh`
    and/or Tilt `scripts/setup-db.sh` (`sesame-idam-db-init` /
    `sesame-idam-apply-migrations`)
- **Redis:** `redis.data.svc.cluster.local:6379`

Local access: LAN proxy / `just port-forward` (postgres often `5433` → Pgpool
`5432`, redis `6379`, both from namespace `data`).

The previous app-local Redis manifest (`redis.yaml`) and its PV were removed to
avoid duplicating the platform data stack. Legacy `postgres-primary` Deployments
are retired — do not wait on them.
