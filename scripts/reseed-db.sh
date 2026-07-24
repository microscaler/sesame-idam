#!/usr/bin/env bash
# Gate B5: one-command wipe + reseed of the Sesame-IDAM store back to the
# known synthetic-sample state. Blast-radius containment depends on this
# being reflex: DROP the app schema, re-apply the generated migrations
# (FK-safe apply_order.txt), re-apply the demo seeds (seed_order.txt).
#
# Targets (RESEED_TARGET):
#   kubectl (default) — the shared cluster's postgres-primary in the `data`
#                       namespace, same exec pattern as scripts/setup-db.sh.
#   docker            — a standalone container (e.g. e2e-sesame-pg on a
#                       build host).
#
# Env:
#   RESEED_CONFIRM=yes            required (destructive!)
#   RESEED_TARGET=kubectl|docker  default kubectl
#   kubectl target: SESAME_IDAM_DB_DATA_NAMESPACE (data),
#                   SESAME_IDAM_POSTGRES_LABEL (app=postgres-primary),
#                   SESAME_IDAM_POSTGRES_CONTAINER (postgres),
#                   database sesame_idam
#   docker target:  RESEED_DOCKER_CONTAINER (e2e-sesame-pg),
#                   RESEED_DB_USER (root), RESEED_DB_NAME (sesame)
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

if [ "${RESEED_CONFIRM:-}" != "yes" ]; then
  echo "❌ Refusing to run: reseed DROPS the sesame_idam schema." >&2
  echo "   Set RESEED_CONFIRM=yes to proceed (just reseed / just reseed-e2e do this)." >&2
  exit 1
fi

TARGET="${RESEED_TARGET:-kubectl}"
DATA_NAMESPACE="${SESAME_IDAM_DB_DATA_NAMESPACE:-data}"
POSTGRES_LABEL="${SESAME_IDAM_POSTGRES_LABEL:-app=postgres-primary}"
POSTGRES_CONTAINER="${SESAME_IDAM_POSTGRES_CONTAINER:-postgres}"
DOCKER_CONTAINER="${RESEED_DOCKER_CONTAINER:-e2e-sesame-pg}"
DOCKER_DB_USER="${RESEED_DB_USER:-root}"
DOCKER_DB_NAME="${RESEED_DB_NAME:-sesame}"

case "${TARGET}" in
  kubectl)
    POSTGRES_POD="$(kubectl get pods -n "${DATA_NAMESPACE}" -l "${POSTGRES_LABEL}" \
      -o jsonpath='{.items[0].metadata.name}')"
    [ -n "${POSTGRES_POD}" ] || { echo "❌ no postgres-primary pod" >&2; exit 1; }
    psql_exec() {
      kubectl exec -i -n "${DATA_NAMESPACE}" "pod/${POSTGRES_POD}" -c "${POSTGRES_CONTAINER}" -- \
        sh -c '
          if [ -n "${POSTGRES_PASSWORD_FILE:-}" ] && [ -r "${POSTGRES_PASSWORD_FILE}" ]; then
            PGPASSWORD="$(cat "${POSTGRES_PASSWORD_FILE}")"
          else
            PGPASSWORD="${POSTGRESQL_PASSWORD:-${POSTGRES_PASSWORD:-}}"
          fi
          export PGPASSWORD
          exec psql -q -h 127.0.0.1 -p 5432 \
            -U "${POSTGRES_USER:-${POSTGRESQL_USERNAME:-postgres}}" \
            -d sesame_idam -v ON_ERROR_STOP=1
        '
    }
    echo "🎯 Target: cluster postgres (${DATA_NAMESPACE}/${POSTGRES_POD}), database sesame_idam"
    ;;
  docker)
    psql_exec() {
      docker exec -i "${DOCKER_CONTAINER}" \
        psql -q -U "${DOCKER_DB_USER}" -d "${DOCKER_DB_NAME}" -v ON_ERROR_STOP=1
    }
    echo "🎯 Target: docker container ${DOCKER_CONTAINER}, database ${DOCKER_DB_NAME}"
    ;;
  *)
    echo "❌ RESEED_TARGET must be kubectl or docker" >&2; exit 1 ;;
esac

echo "💥 Dropping schema sesame_idam (all app data)..."
psql_exec <<'EOF'
DROP SCHEMA IF EXISTS sesame_idam CASCADE;
CREATE SCHEMA sesame_idam;
EOF

echo "📥 Re-applying migrations (FK-safe order)..."
while IFS= read -r rel || [ -n "${rel}" ]; do
  case "${rel}" in \#*|"") continue;; esac
  f="./migrations/${rel}"
  [ -f "${f}" ] || { echo "  ⚠️  missing: ${f}" >&2; continue; }
  { printf 'SET search_path TO sesame_idam, public;\n'; cat "${f}"; } | psql_exec
done < ./migrations/apply_order.txt

echo "🌱 Re-applying seeds (FK-safe order)..."
while IFS= read -r rel || [ -n "${rel}" ]; do
  case "${rel}" in \#*|"") continue;; esac
  f="./microservices/idam/${rel}"
  [ -f "${f}" ] || { echo "  ⚠️  missing: ${f}" >&2; continue; }
  { printf 'SET search_path TO sesame_idam, public;\n'; cat "${f}"; } | psql_exec
done < ./microservices/idam/seed_order.txt

# Grants: only meaningful where the app role exists (cluster); harmless no-op
# guard elsewhere.
echo "🔐 Re-granting DML to sesame_idam role (if present)..."
psql_exec <<'EOF'
DO $$
BEGIN
  IF EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'sesame_idam') THEN
    GRANT USAGE ON SCHEMA sesame_idam TO sesame_idam;
    GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA sesame_idam TO sesame_idam;
    GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA sesame_idam TO sesame_idam;
  END IF;
END $$;
EOF

echo "🔎 Verifying known clean state..."
psql_exec <<'EOF'
SET search_path TO sesame_idam, public;
SELECT 'tables', count(*) FROM information_schema.tables WHERE table_schema = 'sesame_idam';
SELECT 'tenants', count(*) FROM sesame_idam.tenants;
SELECT 'users', count(*) FROM sesame_idam.users;
SELECT 'organizations', count(*) FROM sesame_idam.organizations;
EOF

echo "✅ Reseed complete — store is back to the synthetic-sample baseline."
