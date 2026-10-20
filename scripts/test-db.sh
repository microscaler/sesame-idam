#!/usr/bin/env bash
# Sesame's database tests against a throwaway Postgres and Redis, set up the way production is.
#
#   scripts/test-db.sh                   every test in the workspace, database tests included (none may skip)
#   scripts/test-db.sh -p <package> ...  arguments go to `cargo nextest run`
#
# What "the way production is" means here, because it is what decides whether row-level security holds:
#   - the schema is applied by a migration login acting as the owner group (scripts/sesame-migrate.sh: objects are
#     owned by the group, dev role names rewritten the same way);
#   - the services' login (sesame_runtime) is a MEMBER of the owner group, so like production it is exempt from
#     ordinary RLS on the tables the group owns: only FORCE ROW LEVEL SECURITY protects them. A query that forgets the
#     tenant session therefore behaves here as it does in production;
#   - Redis for token versions (role changes and disables make tokens stale).
# SESAME_TEST_DB_REQUIRED=1 turns every "Postgres not reachable, skipping" into a failure.
#
# Needs initdb/pg_ctl/psql (PostgreSQL 14+) and redis-server on PATH. Nothing outside a temporary directory is touched.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PGPORT="${SESAME_TEST_PGPORT:-55432}"
REDISPORT="${SESAME_TEST_REDISPORT:-56379}"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/sesame-test-db.XXXXXX")"
PASS=test-only-not-a-secret

cleanup() {
  pg_ctl -D "$WORK/pg" -m immediate stop >/dev/null 2>&1 || true
  [ -f "$WORK/redis.pid" ] && kill "$(cat "$WORK/redis.pid")" 2>/dev/null || true
  rm -rf "$WORK"
}
trap cleanup EXIT

for tool in initdb pg_ctl psql redis-server perl; do
  command -v "$tool" >/dev/null || { echo "test-db: $tool is required" >&2; exit 2; }
done

initdb -D "$WORK/pg" -U postgres --auth=trust >/dev/null
pg_ctl -D "$WORK/pg" -o "-p $PGPORT -k $WORK -c listen_addresses=127.0.0.1" -l "$WORK/pg.log" -w start >/dev/null
redis-server --port "$REDISPORT" --bind 127.0.0.1 --save '' --appendonly no --daemonize yes \
  --pidfile "$WORK/redis.pid" --dir "$WORK" >/dev/null

su_psql() { psql -h 127.0.0.1 -p "$PGPORT" -U postgres -X -v ON_ERROR_STOP=1 -q "$@"; }
su_psql -d postgres <<SQL
CREATE ROLE sesame_owner NOLOGIN;
CREATE ROLE sesame_migrator LOGIN PASSWORD '$PASS' IN ROLE sesame_owner;
ALTER ROLE sesame_migrator SET role TO sesame_owner;
CREATE ROLE sesame_runtime LOGIN PASSWORD '$PASS' IN ROLE sesame_owner;
CREATE ROLE rp_directory_owner NOLOGIN BYPASSRLS;
CREATE ROLE rp_directory_read NOLOGIN;
GRANT rp_directory_owner TO sesame_owner;
CREATE DATABASE sesame_idam OWNER sesame_owner;
SQL

export PGPASSWORD="$PASS"
mig_psql() { psql -h 127.0.0.1 -p "$PGPORT" -U sesame_migrator -d sesame_idam -X -v ON_ERROR_STOP=1 -q "$@"; }
mig_psql -c "CREATE SCHEMA IF NOT EXISTS sesame_idam; ALTER DATABASE sesame_idam SET search_path TO sesame_idam, public;"
# scripts/sesame-migrate.sh's rewrite: dev role names -> the owner group
rewrite() {
  perl -pe "s/(?<!search_path )\bTO\s+sesame_idam\b(?![.\w])/TO sesame_owner/gi; s/\bFOR\s+ROLE\s+postgres\b/FOR ROLE sesame_owner/gi" "$1"
}
apply_list() {
  local rel
  while IFS= read -r rel || [ -n "$rel" ]; do
    [[ -z "$rel" || "$rel" =~ ^# ]] && continue
    { echo "SET search_path TO sesame_idam, public;"; rewrite "$2/$rel"; } | mig_psql -f - >/dev/null \
      || { echo "test-db: applying $2/$rel failed" >&2; exit 1; }
  done < "$1"
}
apply_list "$ROOT/migrations/apply_order.txt" "$ROOT/migrations"
apply_list "$ROOT/microservices/idam/seed_environments.txt" "$ROOT/microservices/idam"
# The dev/demo fixtures some tests read (acme's clients, the demo owner): applied as dev does (scripts/setup-db.sh, as
# the database superuser), after the schema, so they are test data and never part of what production runs.
while IFS= read -r rel || [ -n "$rel" ]; do
  [[ -z "$rel" || "$rel" =~ ^# ]] && continue
  [ -f "$ROOT/microservices/idam/$rel" ] || continue
  grep -qxF "$rel" "$ROOT/microservices/idam/seed_environments.txt" && continue   # applied above already
  { echo "SET search_path TO sesame_idam, public;"; cat "$ROOT/microservices/idam/$rel"; } \
    | psql -h 127.0.0.1 -p "$PGPORT" -U postgres -X -q -d sesame_idam -f - >/dev/null 2>"$WORK/seed.err" \
    || true   # as dev applies them: statement by statement, overlaps between fixtures tolerated
  grep -q ERROR "$WORK/seed.err" && echo "test-db: (dev fixture $rel: $(grep -c ERROR "$WORK/seed.err") statement(s) already present)" >&2
done < "$ROOT/microservices/idam/seed_order.txt"
echo "test-db: schema applied (Postgres :$PGPORT, Redis :$REDISPORT)"

export TEST_DB_HOST=127.0.0.1 TEST_DB_PORT="$PGPORT" TEST_DB_USER=sesame_runtime TEST_DB_PASS="$PASS" \
  TEST_DB_NAME=sesame_idam TEST_DB_MIGRATOR=sesame_migrator SESAME_TEST_DB_REQUIRED=1 \
  REDIS_URL="redis://127.0.0.1:$REDISPORT"
cd "$ROOT/microservices"
if [ "$#" -gt 0 ]; then
  cargo +stable nextest run --no-fail-fast "$@"
else
  cargo +stable nextest run --workspace --all-targets --no-fail-fast
fi
