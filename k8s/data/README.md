# Sesame-IDAM data components (Redis; Supabase externalised to microscaler-supabase)

- **persistent-volumes.yaml** — Redis PV only. Postgres/parquet PVs come from microscaler-supabase (see `just supabase-apply`). Kind node: sesame-idam-control-plane. Requires kind-config `extraMounts`: `/tmp/sesame-idam-data` → `/mnt/sesame-idam-data`.
- **redis.yaml** — Redis 7 (Deployment, Service `redis`, PVC). Port-forward: `kubectl port-forward -n sesame-idam svc/redis 6379:6379`.

**Supabase stack (Postgres, etc.):** Apply from microscaler-supabase side-clone: `just supabase-apply`. That applies `k8s/overlays/seasame-idam` from `../microscaler-supabase`, creating namespace `data`, postgres, postgres-meta, etc. Port-forward postgres: `kubectl port-forward -n data svc/postgres 5432:5432`.

**Tilt:** Loads namespace, Redis PV, Redis. Run `just supabase-apply` once before or after cluster is up, then `tilt up`.
