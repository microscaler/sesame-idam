#!/usr/bin/env bash
# Flux database bootstrap entrypoint. This owns only the cluster-level database
# contract: platform app-user credentials, role, database, schema, privileges,
# and login verification. Application migrations and seeds remain a Tilt
# development concern and must never be added to this Job.
set -euo pipefail

PGHOST="${PGHOST:-postgres.data.svc.cluster.local}"
PGPORT="${PGPORT:-5432}"
PGUSER="${PGUSER:-postgres}"
PGDATABASE="${PGDATABASE:-postgres}"
WAIT_SECONDS="${SESAME_IDAM_DB_INIT_WAIT_SECONDS:-300}"
ROLE_NAME="sesame_idam"
DB_NAME="sesame_idam"
SCHEMA_NAME="sesame_idam"

: "${POSTGRES_ADMIN_PASSWORD:?POSTGRES_ADMIN_PASSWORD is required}"
: "${SESAME_IDAM_DB_PASSWORD:?SESAME_IDAM_DB_PASSWORD is required}"
: "${PGPOOL_CUSTOM_USERS:?PGPOOL_CUSTOM_USERS is required}"
: "${PGPOOL_CUSTOM_PASSWORDS:?PGPOOL_CUSTOM_PASSWORDS is required}"

sql_escape() { printf '%s' "$1" | sed "s/'/''/g"; }

validate_pgpool_contract() {
  local -a users passwords
  local index
  IFS=',' read -r -a users <<<"${PGPOOL_CUSTOM_USERS}"
  IFS=',' read -r -a passwords <<<"${PGPOOL_CUSTOM_PASSWORDS}"
  for index in "${!users[@]}"; do
    if [ "${users[$index]}" = "${ROLE_NAME}" ]; then
      if [ "${passwords[$index]:-}" != "${SESAME_IDAM_DB_PASSWORD}" ]; then
        echo "Pgpool and Sesame-IDAM application credentials do not match" >&2
        return 1
      fi
      return 0
    fi
  done
  echo "Pgpool custom users do not contain ${ROLE_NAME}" >&2
  return 1
}

admin_psql() {
  local database="$1"
  shift
  PGPASSWORD="${POSTGRES_ADMIN_PASSWORD}" \
    psql -X -h "${PGHOST}" -p "${PGPORT}" -U "${PGUSER}" \
      -d "${database}" -v ON_ERROR_STOP=1 "$@"
}

wait_for_postgres() {
  local elapsed=0
  echo "Waiting for PostgreSQL at ${PGHOST}:${PGPORT}..."
  until PGPASSWORD="${POSTGRES_ADMIN_PASSWORD}" pg_isready \
    -h "${PGHOST}" -p "${PGPORT}" -U "${PGUSER}" -d postgres >/dev/null 2>&1; do
    if [ "${elapsed}" -ge "${WAIT_SECONDS}" ]; then
      echo "PostgreSQL did not become ready within ${WAIT_SECONDS}s" >&2
      return 1
    fi
    sleep 2
    elapsed=$((elapsed + 2))
  done
}

bootstrap_database() {
  local password_sql
  password_sql="$(sql_escape "${SESAME_IDAM_DB_PASSWORD}")"
  admin_psql postgres <<SQL
DO \$\$
BEGIN
  IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = '${ROLE_NAME}') THEN
    CREATE ROLE ${ROLE_NAME} LOGIN;
  END IF;
END \$\$;
ALTER ROLE ${ROLE_NAME} PASSWORD '${password_sql}';
SELECT 'CREATE DATABASE ${DB_NAME} OWNER ${ROLE_NAME}'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = '${DB_NAME}')\gexec
SQL

  admin_psql "${DB_NAME}" <<SQL
CREATE SCHEMA IF NOT EXISTS ${SCHEMA_NAME} AUTHORIZATION ${ROLE_NAME};
GRANT CONNECT ON DATABASE ${DB_NAME} TO ${ROLE_NAME};
GRANT ALL PRIVILEGES ON SCHEMA ${SCHEMA_NAME} TO ${ROLE_NAME};
GRANT USAGE, CREATE ON SCHEMA public TO ${ROLE_NAME};
ALTER DATABASE ${DB_NAME} SET search_path TO ${SCHEMA_NAME}, public;
SQL
}

grant_application_access() {
  admin_psql "${DB_NAME}" <<SQL
SET search_path TO ${SCHEMA_NAME};
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA ${SCHEMA_NAME} TO ${ROLE_NAME};
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA ${SCHEMA_NAME} TO ${ROLE_NAME};
ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA ${SCHEMA_NAME}
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO ${ROLE_NAME};
ALTER DEFAULT PRIVILEGES FOR ROLE postgres IN SCHEMA ${SCHEMA_NAME}
  GRANT USAGE, SELECT ON SEQUENCES TO ${ROLE_NAME};
SQL
}

verify_application_login() {
  local result
  result="$(PGPASSWORD="${SESAME_IDAM_DB_PASSWORD}" psql -X -h "${PGHOST}" -p "${PGPORT}" \
    -U "${ROLE_NAME}" -d "${DB_NAME}" -Atqc 'SELECT 1')"
  [ "${result}" = "1" ] || {
    echo "Sesame-IDAM database login verification failed" >&2
    return 1
  }
}

validate_pgpool_contract
wait_for_postgres
bootstrap_database
grant_application_access
verify_application_login
echo "Sesame-IDAM role/database bootstrap complete; application migrations remain Tilt-owned"
