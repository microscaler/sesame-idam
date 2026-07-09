# Sesame-IDAM data components

Sesame-IDAM runs **no app-local data stack**. Postgres and Redis are the shared
platform instances in namespace `data`, managed by
[`shared-k8s-cluster`](../../../shared-k8s-cluster/) (`k8s/platform-data/data`):

- **Postgres:** `postgres.data.svc.cluster.local:5432` (primary; read replicas
  `postgres-replica-0` / `postgres-replica-1` in the same namespace). Database
  `sesame_idam` is bootstrapped by `scripts/setup-db.sh` (Tilt
  `sesame-idam-db-init`).
- **Redis:** `redis.data.svc.cluster.local:6379`.

Local access: `just port-forward` (postgres `5432`, redis `6379`, both from
namespace `data`).

The previous app-local Redis manifest (`redis.yaml`) and its PV were removed to
avoid duplicating the platform data stack.
