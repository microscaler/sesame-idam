#!/usr/bin/env bash
# Local/Tilt database administration helper. Flux does not run this script.
#
# Split of ownership (rerp pattern):
#   - Flux Job `scripts/db-init-job.sh` — role, database, schema, privileges, Pgpool contract.
#   - This script's migration-only mode — application migrations, seeds, post-migration grants.
#
# The shared dev cluster runs Bitnami PostgreSQL HA. This script discovers the
# elected primary and connects directly to it for privileged bootstrap work;
# application traffic continues to use Pgpool through the `postgres` Service.
#
# Layout:
#   - Database `sesame_idam` — app data only.
#   - Schema `sesame_idam` — all Sesame-IDAM tables (search_path default for this database).
#   - Role `sesame_idam` — login role matching helm `app.config.database`.
#   - After ./migrations (apply_order.txt), optional microservices/idam/*/impl/seeds/*.sql.
#
# Credentials:
#   - sesame-idam/sesame-idam-db-credentials (SESAME_IDAM_DB_PASSWORD or DB_PASS).
#   - data/postgres-credentials Pgpool custom-user list must include sesame_idam.
#   - SESAME_IDAM_DB_PASSWORD is a break-glass override only.
#
# Optional:
#   SESAME_IDAM_DB_INIT_TIMEOUT (default 600s).
#   SESAME_IDAM_APPLY_MIGRATIONS_ONLY=1 — skip role/DB creation; wait, migrate, GRANT, verify.
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

DATA_NAMESPACE="${SESAME_IDAM_DB_DATA_NAMESPACE:-data}"
APP_NAMESPACE="${SESAME_IDAM_DB_APP_NAMESPACE:-sesame-idam}"
POSTGRES_STATEFULSET="${SESAME_IDAM_POSTGRES_STATEFULSET:-postgres-ha-postgresql}"
POSTGRES_LABEL="${SESAME_IDAM_POSTGRES_LABEL:-app.kubernetes.io/instance=postgres-ha,app.kubernetes.io/component=postgresql}"
POSTGRES_CONTAINER="${SESAME_IDAM_POSTGRES_CONTAINER:-postgresql}"
PGPOOL_SECRET="${SESAME_IDAM_PGPOOL_SECRET:-postgres-credentials}"
PGPOOL_LABEL="${SESAME_IDAM_PGPOOL_LABEL:-app.kubernetes.io/instance=postgres-ha,app.kubernetes.io/component=pgpool}"
PGPOOL_SERVICE="${SESAME_IDAM_PGPOOL_SERVICE:-postgres.data.svc.cluster.local}"
APP_DB_SECRET="${SESAME_IDAM_DB_SECRET:-sesame-idam-db-credentials}"
WAIT_TIMEOUT="${SESAME_IDAM_DB_INIT_TIMEOUT:-600s}"
POSTGRES_POD=""
SESAME_IDAM_DB_PASSWORD="${SESAME_IDAM_DB_PASSWORD:-}"

sql_escape() { printf '%s' "$1" | sed "s/'/''/g"; }

load_sesame_password() {
  if [ -n "${SESAME_IDAM_DB_PASSWORD}" ]; then
    echo "⚠️  Using explicit SESAME_IDAM_DB_PASSWORD override; ensure the SOPS profiles match." >&2
    return 0
  fi

  if ! kubectl get secret "${APP_DB_SECRET}" -n "${APP_NAMESPACE}" >/dev/null 2>&1; then
    echo "❌ Missing ${APP_NAMESPACE}/${APP_DB_SECRET}." >&2
    echo "   Apply Sesame-IDAM's dev SOPS profile first." >&2
    return 1
  fi

  for key in SESAME_IDAM_DB_PASSWORD DB_PASS password; do
    SESAME_IDAM_DB_PASSWORD="$(
      kubectl get secret "${APP_DB_SECRET}" -n "${APP_NAMESPACE}" \
        -o "jsonpath={.data.${key}}" 2>/dev/null | base64 --decode || true
    )"
    if [ -n "${SESAME_IDAM_DB_PASSWORD}" ]; then
      return 0
    fi
  done

  echo "❌ ${APP_NAMESPACE}/${APP_DB_SECRET} has no usable password key." >&2
  return 1
}

validate_pgpool_credentials() {
  local usernames passwords
  local -a username_list password_list
  local index

  if ! kubectl get secret "${PGPOOL_SECRET}" -n "${DATA_NAMESPACE}" >/dev/null 2>&1; then
    echo "❌ Missing ${DATA_NAMESPACE}/${PGPOOL_SECRET}; PostgreSQL HA is not ready." >&2
    return 1
  fi

  usernames="$(kubectl get secret "${PGPOOL_SECRET}" -n "${DATA_NAMESPACE}" -o jsonpath='{.data.usernames}' | base64 --decode)"
  passwords="$(kubectl get secret "${PGPOOL_SECRET}" -n "${DATA_NAMESPACE}" -o jsonpath='{.data.passwords}' | base64 --decode)"
  IFS=',' read -r -a username_list <<<"${usernames}"
  IFS=',' read -r -a password_list <<<"${passwords}"

  for index in "${!username_list[@]}"; do
    if [ "${username_list[$index]}" = "sesame_idam" ]; then
      if [ "${password_list[$index]:-}" != "${SESAME_IDAM_DB_PASSWORD}" ]; then
        echo "❌ Pgpool's sesame_idam credential does not match ${APP_NAMESPACE}/${APP_DB_SECRET}." >&2
        echo "   Reconcile the postgres-ha and sesame-idam SOPS profiles together." >&2
        return 1
      fi
      return 0
    fi
  done

  echo "❌ Pgpool does not contain the sesame_idam custom user." >&2
  echo "   Reconcile the postgres-ha SOPS profile and HelmRelease before database initialization." >&2
  return 1
}

postgres_psql() {
  local database="$1"
  kubectl exec -i -n "${DATA_NAMESPACE}" "pod/${POSTGRES_POD}" -c "${POSTGRES_CONTAINER}" -- \
    sh -c 'PGPASSWORD="$(cat "$POSTGRES_PASSWORD_FILE")" exec psql -h 127.0.0.1 -p 5432 -U "${POSTGRES_USER:-postgres}" -d "$1" -v ON_ERROR_STOP=1' sh "${database}"
}

wait_for_postgres() {
  local pod
  local is_primary

  echo "⏳ Waiting for statefulset/${POSTGRES_STATEFULSET} rollout (${WAIT_TIMEOUT})..."
  kubectl rollout status "statefulset/${POSTGRES_STATEFULSET}" -n "${DATA_NAMESPACE}" --timeout="${WAIT_TIMEOUT}"

  echo "⏳ Waiting for PostgreSQL HA pods Ready (${WAIT_TIMEOUT})..."
  kubectl wait --for=condition=ready pod -l "${POSTGRES_LABEL}" -n "${DATA_NAMESPACE}" --timeout="${WAIT_TIMEOUT}" >/dev/null

  echo "🔎 Discovering the elected PostgreSQL primary..."
  while IFS= read -r pod; do
    [ -z "${pod}" ] && continue
    is_primary="$(
      kubectl exec -n "${DATA_NAMESPACE}" "pod/${pod}" -c "${POSTGRES_CONTAINER}" -- \
        sh -c 'PGPASSWORD="$(cat "$POSTGRES_PASSWORD_FILE")" psql -h 127.0.0.1 -U "${POSTGRES_USER:-postgres}" -d postgres -Atqc "SELECT NOT pg_is_in_recovery()"' \
        2>/dev/null || true
    )"
    if [ "${is_primary}" = "t" ]; then
      POSTGRES_POD="${pod}"
      echo "✅ PostgreSQL primary: ${POSTGRES_POD}"
      return 0
    fi
  done < <(kubectl get pods -n "${DATA_NAMESPACE}" -l "${POSTGRES_LABEL}" -o jsonpath='{range .items[*]}{.metadata.name}{"\n"}{end}')

  echo "❌ No elected PostgreSQL primary was discoverable." >&2
  return 1
}

bootstrap_sesame_idam_role_and_db() {
  echo "⏳ Creating role sesame_idam, database sesame_idam, schema sesame_idam (if missing)..."
  local password_sql
  password_sql="$(sql_escape "${SESAME_IDAM_DB_PASSWORD}")"
  postgres_psql postgres <<EOF
-- Cluster login role for Sesame-IDAM apps (matches Helm database.user)
DO \$\$
BEGIN
  IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'sesame_idam') THEN
    EXECUTE format('CREATE ROLE sesame_idam LOGIN PASSWORD %L', '${password_sql}');
  ELSE
    EXECUTE format('ALTER ROLE sesame_idam PASSWORD %L', '${password_sql}');
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
      postgres_psql sesame_idam < "${migration_file}"
    }
    if [ -f ./migrations/apply_order.txt ]; then
      # Written by `cargo run -p sesame_idam_migrator` — FK-safe order across services.
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
# sesame_idam_migrator` via lifeguard-migrate's write_seed_order_file). Falls back to alphabetical
# path order for back-compat.
apply_seeds_from_disk() {
  local count
  count="$(find ./microservices -path '*/impl/seeds/*.sql' 2>/dev/null | wc -l | tr -d ' ')"
  if [ -z "${count}" ] || [ "${count}" = "0" ]; then
    return 0
  fi
  apply_one_seed() {
    local seed_file="$1"
    echo "  -> Applying ${seed_file}..."
    postgres_psql sesame_idam < "${seed_file}"
  }
  if [ -f ./microservices/idam/seed_order.txt ]; then
    echo "📥 Applying per-microservice seed SQL (microservices/idam/seed_order.txt, FK-ordered)..."
    while IFS= read -r rel || [ -n "${rel}" ]; do
      [[ -z "${rel}" || "${rel}" =~ ^# ]] && continue
      seed_file="./microservices/idam/${rel}"
      if [ -f "${seed_file}" ]; then
        apply_one_seed "${seed_file}"
      else
        echo "  ⚠️  seed_order.txt lists missing file: ${seed_file}" >&2
      fi
    done < ./microservices/idam/seed_order.txt
  else
    echo "📥 Applying per-microservice seed SQL (microservices/idam/*/impl/seeds/*.sql, alphabetical)..."
    while IFS= read -r -d '' seed_file; do
      apply_one_seed "${seed_file}"
    done < <(find ./microservices -path '*/impl/seeds/*.sql' -print0 2>/dev/null | sort -z)
  fi
}

grant_sesame_idam_dml() {
  # Migrations run as superuser (postgres); tables are owned by postgres. The app role sesame_idam
  # has schema USAGE/CREATE but not automatic DML on those tables — without GRANT, microservices
  # get Postgres errors that surface as Display "db error".
  echo "🔐 GRANT DML on sesame_idam schema objects to role sesame_idam..."
  postgres_psql sesame_idam <<'EOF'
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

verify_app_login_on_primary() {
  local result
  echo "🔐 Verifying the Sesame-IDAM login on the elected primary..."
  if ! result="$(
    printf '%s\n' "${SESAME_IDAM_DB_PASSWORD}" | \
      kubectl exec -i -n "${DATA_NAMESPACE}" "pod/${POSTGRES_POD}" -c "${POSTGRES_CONTAINER}" -- \
        sh -c 'IFS= read -r PGPASSWORD; export PGPASSWORD; psql -h 127.0.0.1 -p 5432 -U sesame_idam -d sesame_idam -Atqc "SELECT 1"'
  )"; then
    echo "❌ Sesame-IDAM cannot authenticate on the primary." >&2
    return 1
  fi
  if [ "${result}" != "1" ]; then
    echo "❌ Primary verification returned an unexpected result." >&2
    return 1
  fi
  echo "✅ Sesame-IDAM login verified on the primary."
}

verify_pgpool_connection() {
  local pgpool_pod
  local result

  # Prefer a Ready Pgpool pod (CrashLoop/Running-but-not-Ready cannot exec).
  # Avoid `... | head` under pipefail — SIGPIPE from head exits the script.
  pgpool_pod=""
  while IFS= read -r pod; do
    [ -z "${pod}" ] && continue
    if kubectl get pod -n "${DATA_NAMESPACE}" "${pod}" \
      -o jsonpath='{.status.conditions[?(@.type=="Ready")].status}' 2>/dev/null | grep -qx True; then
      pgpool_pod="${pod}"
      break
    fi
  done < <(kubectl get pods -n "${DATA_NAMESPACE}" -l "${PGPOOL_LABEL}" \
    -o jsonpath='{range .items[*]}{.metadata.name}{"\n"}{end}' 2>/dev/null || true)

  if [ -z "${pgpool_pod}" ]; then
    echo "⚠️  No Ready Pgpool pod — skipping Pgpool path verify (apps use ${PGPOOL_SERVICE})." >&2
    echo "    Often caused by PostgreSQL max_connections exhaustion; check postgres-ha-pgpool pods." >&2
    verify_app_login_on_primary
    return $?
  fi

  echo "🔐 Verifying the Sesame-IDAM login through Pgpool (${pgpool_pod})..."
  if ! result="$(
    printf '%s\n' "${SESAME_IDAM_DB_PASSWORD}" | \
      kubectl exec -i -n "${DATA_NAMESPACE}" "pod/${pgpool_pod}" -c pgpool -- \
        sh -c 'IFS= read -r PGPASSWORD; export PGPASSWORD; psql -h "$1" -p 5432 -U sesame_idam -d sesame_idam -Atqc "SELECT 1"' sh "${PGPOOL_SERVICE}"
  )"; then
    echo "⚠️  Pgpool login failed; falling back to primary verification." >&2
    verify_app_login_on_primary
    return $?
  fi
  if [ "${result}" != "1" ]; then
    echo "⚠️  Pgpool verification returned an unexpected result; falling back to primary." >&2
    verify_app_login_on_primary
    return $?
  fi
  echo "✅ Sesame-IDAM login verified through Pgpool."
}

load_sesame_password
validate_pgpool_credentials

if [ "${SESAME_IDAM_APPLY_MIGRATIONS_ONLY:-0}" = "1" ]; then
  echo "📌 SESAME_IDAM_APPLY_MIGRATIONS_ONLY=1 — apply migration SQL files to cluster + GRANTs (no role/DB bootstrap)."
  echo "    Run once: sesame-idam-db-init (or Flux Job). After sesame-idam-migrate regenerates SQL, re-run this."
  wait_for_postgres
  apply_migrations_from_disk
  apply_seeds_from_disk
  grant_sesame_idam_dml
  verify_pgpool_connection
  echo "✅ Migrations applied to database sesame_idam."
  exit 0
fi

echo "🚀 Initializing Sesame-IDAM database, role, and schema..."
wait_for_postgres
bootstrap_sesame_idam_role_and_db
apply_migrations_from_disk
apply_seeds_from_disk
grant_sesame_idam_dml
verify_pgpool_connection

echo "✅ Sesame-IDAM database + schema setup complete."
