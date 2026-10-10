#!/usr/bin/env bash
# Apply Sesame-IDAM's schema to an environment's database: the migration Job of a release. Dev uses
# scripts/setup-db.sh (Tilt).
#
#   DATABASE_URL_DIRECT  the migration login, straight to the primary (not a transaction pooler: DDL, session settings)
#   SESAME_SCHEMA        schema (default sesame_idam)
#   SESAME_SEEDS         env (default): the seeds of seed_environments.txt; dev: every seed of seed_order.txt
#   SESAME_RELEASE       recorded in the ledger (the chart sets the image tag)
#
# The login acts as the environment's owner group, so everything created here is owned by that group and every
# service login of the environment (a member of it) can use it. The SQL is written for dev, where that role is
# called sesame_idam and objects are created by postgres: see rewrite() below. Every file is idempotent and applied
# on every run, as in dev; a ledger records what each release applied.
#
# Roles outside the database (migrations/rls/*_rp_directory.sql: rp_directory_owner NOLOGIN BYPASSRLS,
# rp_directory_read) are the platform's: it creates them and grants rp_directory_owner to the owner group. This
# script never needs superuser.
set -euo pipefail

: "${DATABASE_URL_DIRECT:?DATABASE_URL_DIRECT is required}"
SCHEMA="${SESAME_SCHEMA:-sesame_idam}"
SEEDS="${SESAME_SEEDS:-env}"
WAIT_SECONDS="${SESAME_MIGRATE_WAIT_SECONDS:-300}"
RELEASE="${SESAME_RELEASE:-unknown}"
case "$SEEDS" in env|dev) ;; *) echo "SESAME_SEEDS must be env or dev, not $SEEDS" >&2; exit 2 ;; esac
[[ "$SCHEMA" =~ ^[a-z_][a-z0-9_]*$ ]] || { echo "SESAME_SCHEMA must be a plain identifier" >&2; exit 2; }

cd /sesame

psql_db() { psql "$DATABASE_URL_DIRECT" -X -v ON_ERROR_STOP=1 -qAt "$@"; }

elapsed=0
until psql_db -c 'select 1' >/dev/null 2>&1; do
  [ "$elapsed" -ge "$WAIT_SECONDS" ] && { echo "database not reachable after ${WAIT_SECONDS}s" >&2; psql_db -c 'select 1'; exit 1; }
  sleep 3; elapsed=$((elapsed + 3))
done

ROLE=$(psql_db -c 'select current_user')
DB=$(psql_db -c 'select current_database()')
[[ "$ROLE" =~ ^[a-z_][a-z0-9_]*$ ]] || { echo "unexpected role name" >&2; exit 1; }
echo "applying the schema to database $DB as $ROLE (session user $(psql_db -c 'select session_user')), release $RELEASE, seeds $SEEDS"

psql_db <<SQL
CREATE SCHEMA IF NOT EXISTS ${SCHEMA};
ALTER DATABASE "${DB}" SET search_path TO ${SCHEMA}, public;
CREATE TABLE IF NOT EXISTS ${SCHEMA}.sesame_schema_applied (
  file       text PRIMARY KEY,
  sha256     text NOT NULL,
  release    text NOT NULL,
  applied_at timestamptz NOT NULL DEFAULT now()
);
SQL

# Dev role names -> this environment's owner group: "TO sesame_idam" (GRANT ... TO; never the schema:
# sesame_idam.<table>, search_path) and "FOR ROLE postgres" (ALTER DEFAULT PRIVILEGES: dev creates objects as
# postgres; here the owner group creates them and may set only its own defaults).
rewrite() {
  perl -pe "s/(?<!search_path )\bTO\s+sesame_idam\b(?![.\w])/TO ${ROLE}/gi; s/\bFOR\s+ROLE\s+postgres\b/FOR ROLE ${ROLE}/gi" "$1"
}

apply() {
  local f="$1" sum
  sum=$(sha256sum "$f" | cut -c1-64)
  echo "  $f"
  { echo "SET search_path TO ${SCHEMA}, public;"; rewrite "$f"; } | psql_db -f - >/dev/null
  psql_db -v file="$f" -v sum="$sum" -v release="$RELEASE" <<SQL >/dev/null
INSERT INTO ${SCHEMA}.sesame_schema_applied (file, sha256, release) VALUES (:'file', :'sum', :'release')
ON CONFLICT (file) DO UPDATE SET sha256 = excluded.sha256, release = excluded.release, applied_at = now();
SQL
}

listed() {
  # $1 list file, $2 prefix, $3 strict|lenient: every non-comment line, in order. strict: a missing file fails the
  # run; lenient (dev seeds, as setup-db.sh): warned and skipped.
  local rel
  while IFS= read -r rel || [ -n "$rel" ]; do
    [[ -z "$rel" || "$rel" =~ ^# ]] && continue
    if [ ! -f "$2/$rel" ]; then
      [ "$3" = strict ] && { echo "$1 lists $2/$rel, which is not in the image" >&2; exit 1; }
      echo "  (skipped: $1 lists $2/$rel, which does not exist)" >&2
      continue
    fi
    echo "$2/$rel"
  done < "$1"
}

echo "migrations (migrations/apply_order.txt):"
# Command substitution, not <(...): a failed listing must stop the run.
list=$(listed migrations/apply_order.txt migrations strict)
for f in $list; do apply "$f"; done

if [ "$SEEDS" = dev ]; then
  echo "seeds, dev (microservices/idam/seed_order.txt):"
  list=$(listed microservices/idam/seed_order.txt microservices/idam lenient)
else
  echo "seeds, every environment (microservices/idam/seed_environments.txt):"
  list=$(listed microservices/idam/seed_environments.txt microservices/idam strict)
fi
for f in $list; do apply "$f"; done

echo "schema applied: $(psql_db -c "select count(*) from ${SCHEMA}.sesame_schema_applied where release = '${RELEASE//\'/}'") file(s), release $RELEASE"
