#!/usr/bin/env bash
# Apply Sesame-IDAM's schema to an environment's database: the migration Job of a release. Dev uses
# scripts/setup-db.sh (Tilt).
#
#   DATABASE_URL_DIRECT  the migration login, straight to the primary (not a transaction pooler: DDL, session settings)
#   SESAME_SCHEMA        schema (default sesame_idam)
#   (seeds)              the seeds of seed_environments.txt; demo seeds are not in the image (dev: setup-db.sh)
#   SESAME_RELEASE       recorded in the ledger (the chart sets the image tag)
#
# The login acts as the environment's owner group, so everything created here is owned by that group and every
# service login of the environment (a member of it) can use it. The SQL is written for dev, where that role is
# called sesame_idam and objects are created by postgres: see rewrite() below. Every file is idempotent and applied
# on every run, as in dev; a ledger records what each release applied.
#
# Logs, one line per event so each is one Cloud Logging entry (stdout INFO, stderr ERROR): a start line (release,
# psql version, database, host and port, never the password; the number of files, all of which run), "start" and
# "applied <file> (<ms> ms)" per file, and on a failure one FAILED line on stderr with the file, the statement's
# number and first line in the file, its first 200 characters and the server's error (psql's verbose report on one
# line: SQLSTATE, message, DETAIL, HINT, LINE + caret for the position, CONTEXT), then exit 1. Server NOTICEs
# ("already exists, skipping") are not shown: they went to stderr and showed as errors on every run.
#
# Roles outside the database (migrations/rls/*_rp_directory.sql: rp_directory_owner NOLOGIN BYPASSRLS,
# rp_directory_read) are the platform's: it creates them and grants rp_directory_owner to the owner group. This
# script never needs superuser.
set -euo pipefail

: "${DATABASE_URL_DIRECT:?DATABASE_URL_DIRECT is required}"
SCHEMA="${SESAME_SCHEMA:-sesame_idam}"
WAIT_SECONDS="${SESAME_MIGRATE_WAIT_SECONDS:-300}"
RELEASE="${SESAME_RELEASE:-unknown}"
if [ -n "${SESAME_SEEDS:-}" ] && [ "${SESAME_SEEDS}" != env ]; then
  echo "SESAME_SEEDS=${SESAME_SEEDS}: this image applies only the seeds every environment gets; demo seeds are dev only (scripts/setup-db.sh)" >&2
  exit 2
fi
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
SESSION_USER=$(psql_db -c 'select session_user')
HOSTPORT=$(psql_db -c '\echo :HOST port :PORT')   # psql's own variables: never the password

psql_db <<SQL
SET client_min_messages TO warning;
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

now_ms() { date +%s%3N; }

# Which statement psql stopped at, from the SQL it was given (stdin) and the line it reported (the line of the
# statement's closing ;): "<number>\t<first line>\t<first 200 characters>". A statement ends at a ; outside quotes,
# dollar quotes and comments. The input's first line is the preamble (2 statements), so the number and the line are
# the file's own.
LOCATE_PL=$(cat <<'PERL'
use strict; use warnings;
my $at = shift; local $/; my $sql = <STDIN>;
my ($i, $n, $line, $len) = (0, 0, 1, length $sql);
my ($start, $sline);
while ($i < $len) {
  my $c = substr($sql, $i, 1);
  if ($c eq "\n") { $line++; $i++; next }
  if ($c =~ /\s/) { $i++; next }
  if (substr($sql, $i, 2) eq '--') { my $e = index($sql, "\n", $i); $i = $e < 0 ? $len : $e; next }
  if (!defined $sline) { $sline = $line; $start = $i }
  my $e;
  if (substr($sql, $i, 2) eq '/*') { $e = index($sql, '*/', $i + 2); $e = $e < 0 ? $len : $e + 2 }
  elsif ($c eq "'" || $c eq '"') { $e = index($sql, $c, $i + 1); $e = $e < 0 ? $len : $e + 1 }
  elsif (substr($sql, $i, 64) =~ /^(\$[A-Za-z_]*\$)/) {
    my $t = $1; $e = index($sql, $t, $i + length $t); $e = $e < 0 ? $len : $e + length $t;
  }
  if (defined $e) { $line += (substr($sql, $i, $e - $i) =~ tr/\n//); $i = $e; next }
  if ($c eq ';') { $n++; last if $line >= $at; undef $sline; $i++; next }
  $i++;
}
my $end = $i < $len ? $i + 1 : $len;
$n++ if $i >= $len && defined $sline;
my $stmt = defined $sline ? substr($sql, $start, $end - $start) : '';
$stmt =~ s/\s+/ /g; $stmt =~ s/^ | $//g;
printf "%d\t%d\t%s\n", $n - 2, ($sline // $line) - 1, length($stmt) > 200 ? substr($stmt, 0, 200) . '...' : $stmt;
PERL
)

apply() {
  local f="$1" sum t0 err sql at where pos
  sum=$(sha256sum "$f" | cut -c1-64)
  echo "  start   $f"
  t0=$(now_ms)
  err=$(mktemp); sql=$(mktemp)
  { echo "SET search_path TO ${SCHEMA}, public; SET client_min_messages TO warning;"; rewrite "$f"; } > "$sql"
  if ! psql_db -v VERBOSITY=verbose -f - < "$sql" >/dev/null 2>"$err"; then
    # psql:<stdin>:<line>: ERROR:  <SQLSTATE>: <message>, then DETAIL / HINT / LINE + caret / CONTEXT / LOCATION.
    at=$(sed -n 's/^psql:<stdin>:\([0-9]*\): ERROR:.*/\1/p' "$err" | head -1)
    where=$(perl -e "$LOCATE_PL" "${at:-999999999}" < "$sql")
    # The server's position, from psql's "LINE <n>: <text>" and the caret under it (n: line within the statement).
    pos=$(awk '/^LINE [0-9]+: / { n = $2; sub(/:$/, "", n); getline c; print "statement line " n ", column " index(c, "^") - length("LINE " n ": "); exit }' "$err")
    printf 'FAILED file=%s statement=%s line=%s after_ms=%s position="%s" error="%s" statement_head="%s"\n' \
      "$f" "$(cut -f1 <<<"$where")" "$(cut -f2 <<<"$where")" "$(( $(now_ms) - t0 ))" "${pos:-none}" \
      "$(grep -v '^ *\^ *$' "$err" | sed -e 's/^psql:<stdin>:[0-9]*: //' -e "s/\"/'/g" -e 's/  */ /g' | paste -sd'|' | sed 's/|/ | /g')" \
      "$(cut -f3- <<<"$where" | sed "s/\"/'/g")" >&2
    rm -f "$err" "$sql"
    exit 1
  fi
  [ -s "$err" ] && cat "$err" >&2   # warnings
  rm -f "$err" "$sql"
  psql_db -v file="$f" -v sum="$sum" -v release="$RELEASE" <<SQL >/dev/null
INSERT INTO ${SCHEMA}.sesame_schema_applied (file, sha256, release) VALUES (:'file', :'sum', :'release')
ON CONFLICT (file) DO UPDATE SET sha256 = excluded.sha256, release = excluded.release, applied_at = now();
SQL
  echo "  applied $f ($(( $(now_ms) - t0 )) ms)"
}

listed() {
  # $1 list file, $2 prefix, $3 strict|lenient: every non-comment line, in order. strict: a missing file fails the
  # run; lenient: warned and skipped.
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

# Command substitution, not <(...): a failed listing must stop the run.
migrations=$(listed migrations/apply_order.txt migrations strict)
seeds=$(listed microservices/idam/seed_environments.txt microservices/idam strict)
pending=$(printf '%s\n' $migrations $seeds | grep -c . || true)
echo "sesame-migrate release=$RELEASE $(psql --version): database $DB on $HOSTPORT as $ROLE (session user $SESSION_USER), $pending file(s) pending (every file runs on every run)"

echo "migrations (migrations/apply_order.txt):"
for f in $migrations; do apply "$f"; done

echo "seeds, every environment (microservices/idam/seed_environments.txt):"
for f in $seeds; do apply "$f"; done

echo "schema applied: $(psql_db -c "select count(*) from ${SCHEMA}.sesame_schema_applied where release = '${RELEASE//\'/}'") file(s), release $RELEASE"
