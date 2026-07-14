#!/usr/bin/env bash
# Setup script for Sesame-IDAM PostgreSQL database + schema (in-cluster; PostgreSQL listens on 127.0.0.1:5432 in the pod).
# Waits for deployment/postgres-primary and the main container before kubectl exec (avoids "container not found").
#
# Layout:
#   - Database `sesame_idam` — app data only (Supabase stack uses database `postgres` on the same server).
#   - Schema `sesame_idam` — all Sesame-IDAM tables (search_path default for this database).
#   - Role `sesame_idam` — login role matching helm `app.config.database` (password from env below).
#   - After ./migrations (apply_order.txt), optional microservices/idam/*/impl/seeds/*.sql (not Lifeguard output).
#
# Optional:
#   SESAME_IDAM_DB_INIT_TIMEOUT (default 600s), SESAME_IDAM_DB_PASSWORD (must match helm dev password).
#   SESAME_IDAM_APPLY_MIGRATIONS_ONLY=1 — skip role/DB creation; only wait for postgres, apply ./migrations (apply_order.txt), then GRANTs.
#     Use after `cargo run -p sesame_idam_migrator` (or Tilt `sesame-idam-migrate`) when SQL files already exist.
set -euo pipefail

NS=data
DEPLOY=postgres-primary
WAIT_TIMEOUT="${SESAME_IDAM_DB_INIT_TIMEOUT:-600s}"
# Must match helm/sesame-idam-microservice/values.yaml app.config.database.password for local Kind.
SESAME_IDAM_DB_PASSWORD="${SESAME_IDAM_DB_PASSWORD:-dev_password_change_in_prod}"

sql_escape() { printf '%s' "$1" | sed "s/'/''/g"; }
PW_SQL=$(sql_escape "${SESAME_IDAM_DB_PASSWORD}")

wait_for_postgres() {
  echo "⏳ Waiting for ${DEPLOY} rollout (${WAIT_TIMEOUT})..."
  kubectl rollout status "deployment/${DEPLOY}" -n "${NS}" --timeout="${WAIT_TIMEOUT}"

  echo "⏳ Waiting for postgres pod Ready (${WAIT_TIMEOUT})..."
  kubectl wait --for=condition=ready pod -l 'app in (postgres, postgres-primary)' -n "${NS}" --timeout="${WAIT_TIMEOUT}" >/dev/null
}

bootstrap_sesame_idam_role_and_db() {
  # Main container name is "postgres" (microscaler-supabase k8s/data/postgres.yaml). Always pass -c postgres.
  # $POSTGRES_USER is the cluster superuser inside the pod.
  echo "⏳ Creating role sesame_idam, database sesame_idam, schema sesame_idam (if missing)..."
  kubectl exec -i -n "${NS}" "deployment/${DEPLOY}" -c postgres -- \
    sh -c 'env PGPASSWORD="$POSTGRESQL_PASSWORD" psql -h 127.0.0.1 -p 5432 -U postgres -d postgres -v ON_ERROR_STOP=1' <<EOF
-- Cluster login role for Sesame-IDAM apps (matches Helm database.user)
DO \$\$
BEGIN
  IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'sesame_idam') THEN
    EXECUTE format('CREATE ROLE sesame_idam LOGIN PASSWORD %L', '${PW_SQL}');
  ELSE
    EXECUTE format('ALTER ROLE sesame_idam PASSWORD %L', '${PW_SQL}');
  END IF;
END \$\$;

SELECT 'CREATE DATABASE sesame_idam OWNER sesame_idam'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'sesame_idam')\gexec

\c sesame_idam

CREATE SCHEMA IF NOT EXISTS sesame_idam;
GRANT CONNECT ON DATABASE sesame_idam TO sesame_idam;
GRANT ALL PRIVILEGES ON SCHEMA sesame_idam TO sesame_idam;
ALTER SCHEMA sesame_idam OWNER TO sesame_idam;
-- Allow extensions / shared objects that still use public if needed
GRANT USAGE ON SCHEMA public TO sesame_idam;
GRANT CREATE ON SCHEMA public TO sesame_idam;

ALTER DATABASE sesame_idam SET search_path TO sesame_idam, public;
EOF
}

apply_migrations_from_disk() {
  if [ -d ./migrations ]; then
    echo "📥 Applying Lifeguard migration SQL (search_path=sesame_idam from ALTER DATABASE)..."
    apply_one() {
      local migration_file="$1"
      echo "  -> Applying ${migration_file}..."
      cat "${migration_file}" | kubectl exec -i -n "${NS}" "deployment/${DEPLOY}" -c postgres -- \
        sh -c 'env PGPASSWORD="$POSTGRESQL_PASSWORD" psql -h 127.0.0.1 -p 5432 -U postgres -d sesame_idam -v ON_ERROR_STOP=1'
    }
    if [ -f ./migrations/apply_order.txt ]; then
      # Written by `cargo run -p sesame_idam_migrator` — FK-safe order across services (no path sort).
      while IFS= read -r rel || [ -n "${rel}" ]; do
        [[ -z "${rel}" || "${rel}" =~ ^# ]] && continue
        migration_file="./migrations/${rel}"
        if [ -f "${migration_file}" ]; then
          apply_one "${migration_file}"
        else
          echo "  ⚠️  apply_order.txt lists missing file: ${migration_file}" >&2
        fi
      done < ./migrations/apply_order.txt
    else
      echo "  (no apply_order.txt — falling back to lexicographic path sort; run sesame-idam migrator to generate)"
      while IFS= read -r -d '' migration_file; do
        apply_one "${migration_file}"
      done < <(find ./migrations -name "*.sql" -print0 | sort -z)
    fi
  else
    echo "📥 No ./migrations directory; skipping SQL file ingest."
  fi
}

# Optional dev/demo data — not produced by lifeguard-migrate; one directory per microservice (impl/seeds/).
#
# Ordering: uses microservices/idam/seed_order.txt when present (produced by `cargo run -p
# sesame_idam_migrator` via lifeguard-migrate's write_seed_order_file — FK-aware across services,
# so tables are populated in correct FK order). Falls back to alphabetical path order for back-compat.
#
# NOTE: lifeguard-migrate enforces a strict `YYYYMMDDHHMMSS_<slug>.sql` filename convention on
# seeds. Files without that prefix are silently skipped from seed_order.txt (with a stderr
# warning during generation) and therefore will NOT be applied by this script when seed_order.txt
# is present.
apply_seeds_from_disk() {
  local count
  count="$(find ./microservices -path '*/impl/seeds/*.sql' 2>/dev/null | wc -l | tr -d ' ')"
  if [ -z "${count}" ] || [ "${count}" = "0" ]; then
    return 0
  fi
  apply_one_seed() {
    local seed_file="$1"
    echo "  -> Applying ${seed_file}..."
    cat "${seed_file}" | kubectl exec -i -n "${NS}" "deployment/${DEPLOY}" -c postgres -- \
      sh -c 'env PGPASSWORD="$POSTGRESQL_PASSWORD" psql -h 127.0.0.1 -p 5432 -U postgres -d sesame_idam -v ON_ERROR_STOP=1'
  }
  if [ -f ./microservices/idam/seed_order.txt ]; then
    echo "📥 Applying per-microservice seed SQL (microservices/idam/seed_order.txt, FK-ordered)..."
    while IFS= read -r rel || [ -n "${rel}" ]; do
      [[ -z "${rel}" || "${rel}" =~ ^# ]] && continue
      # seed_order.txt paths are relative to microservices/idam (the migrator's seeds root)
      seed_file="./microservices/idam/${rel}"
      if [ -f "${seed_file}" ]; then
        apply_one_seed "${seed_file}"
      else
        echo "  ⚠️  seed_order.txt lists missing file: ${seed_file}" >&2
      fi
    done < ./microservices/idam/seed_order.txt
  else
    echo "📥 Applying per-microservice seed SQL (microservices/idam/*/impl/seeds/*.sql, alphabetical — run sesame-idam migrator to generate seed_order.txt)..."
    while IFS= read -r -d '' seed_file; do
      apply_one_seed "${seed_file}"
    done < <(find ./microservices -path '*/impl/seeds/*.sql' -print0 2>/dev/null | sort -z)
  fi
}

grant_sesame_idam_dml() {
  # Migrations run as superuser (`postgres`); tables are owned by `postgres`. The app role `sesame_idam`
  # has schema USAGE/CREATE but not automatic DML on those tables — without GRANT, microservices
  # get Postgres errors that tokio-postgres surfaces only as Display "db error".
  echo "🔐 GRANT DML on sesame_idam schema objects to role sesame_idam..."
  kubectl exec -i -n "${NS}" "deployment/${DEPLOY}" -c postgres -- \
    sh -c 'env PGPASSWORD="$POSTGRESQL_PASSWORD" psql -h 127.0.0.1 -p 5432 -U postgres -d sesame_idam -v ON_ERROR_STOP=1' <<'EOF'
SET search_path TO sesame_idam;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA sesame_idam TO sesame_idam;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA sesame_idam TO sesame_idam;
ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA sesame_idam GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO sesame_idam;
ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA sesame_idam GRANT USAGE, SELECT ON SEQUENCES TO sesame_idam;
GRANT EXECUTE ON FUNCTION public.rls_set_session(text, uuid, uuid, text, jsonb, jsonb, text, text) TO sesame_idam;
GRANT EXECUTE ON FUNCTION public.rls_set_pre_auth_tenant(text) TO sesame_idam;
GRANT EXECUTE ON FUNCTION public.sesame_current_tenant_id() TO sesame_idam;
GRANT EXECUTE ON FUNCTION public.sesame_rls_contract_version() TO sesame_idam;
EOF
}

if [ "${SESAME_IDAM_APPLY_MIGRATIONS_ONLY:-0}" = "1" ]; then
  echo "📌 SESAME_IDAM_APPLY_MIGRATIONS_ONLY=1 — apply migration SQL files to cluster + GRANTs (no role/DB bootstrap)."
  echo "    Run once: sesame-idam-db-init. After sesame-idam-migrate regenerates SQL, run this resource or re-run with the env set."
  wait_for_postgres
  apply_migrations_from_disk
  apply_seeds_from_disk
  grant_sesame_idam_dml
  echo "✅ Migrations applied to database sesame_idam."
  exit 0
fi

echo "🚀 Initializing Sesame-IDAM database, role, and schema..."
wait_for_postgres
bootstrap_sesame_idam_role_and_db
apply_migrations_from_disk
apply_seeds_from_disk
grant_sesame_idam_dml

echo "✅ Sesame-IDAM database + schema setup complete."
