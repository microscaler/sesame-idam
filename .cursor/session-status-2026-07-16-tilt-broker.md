# Session status — Sesame Tilt / HA Postgres (2026-07-16)

DONE:
- Broker/pact restore; ruby image tag fix; Tilt active
- setup-db.sh rewritten for postgres-ha (rerp pattern)
- sesame-idam-db-init Tilt resource updateStatus=ok
- Migrations/seeds/grants applied; app login verified on primary

OPEN:
- postgres-ha-pgpool CrashLoop: "too many clients already" — apps via
  postgres.data.svc.cluster.local will fail until max_connections / pool sizes fixed
