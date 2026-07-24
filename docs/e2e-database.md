# e2e database: migrating + seeding a standalone Postgres container

The BDD suites run against any Postgres reachable via `TEST_DB_*` env (e.g.
the `e2e-sesame-pg` docker container on a build host, port 15434). A freshly
bootstrapped container has only the tables the code paths auto-touched —
`users`, `tenants`, `app_passwords` — which fails every test needing the org
/ roles / oauth tables or the hauliage demo seeds
(`principal_effective_db`, `account_first_onboarding`, …).

Apply the same artifacts the cluster's `scripts/setup-db.sh` uses, directly:

```bash
cd sesame-idam

# 1. Migrations in FK-safe order (idempotent CREATE IF NOT EXISTS). The
#    session search_path prefix matters: a few migrations use unqualified
#    table names (same guarantee scripts/setup-db.sh now applies per file).
while IFS= read -r rel; do case "$rel" in \#*|"") continue;; esac
  { echo "SET search_path TO sesame_idam, public;"; cat "migrations/$rel"; } \
    | docker exec -i e2e-sesame-pg psql -q -U root -d sesame -v ON_ERROR_STOP=1
done < migrations/apply_order.txt

# 2. Demo seeds (hauliage users/orgs/roles, platform tenants) in FK order.
while IFS= read -r rel; do case "$rel" in \#*|"") continue;; esac
  { echo "SET search_path TO sesame_idam, public;"; cat "microservices/idam/$rel"; } \
    | docker exec -i e2e-sesame-pg psql -q -U root -d sesame -v ON_ERROR_STOP=1
done < microservices/idam/seed_order.txt

# Optional: also set it database-wide so ad-hoc psql sessions resolve
# unqualified names the same way.
docker exec e2e-sesame-pg psql -U root -d sesame \
  -c "ALTER DATABASE sesame SET search_path TO sesame_idam, public;"
```

Then run the suite:

```bash
TEST_DB_PORT=15434 TEST_DB_USER=root TEST_DB_PASS=test TEST_DB_NAME=sesame \
  cargo nextest run
```

Notes:

- The search_path prefix is load-bearing: `org-mgmt/20260516192347_permissions.sql`,
  `api-keys/20260516192347_api_keys.sql`, and
  `org-mgmt/20260713065042_organizations.sql` reference tables without the
  `sesame_idam.` prefix. `scripts/setup-db.sh` prepends the same session-level
  `SET search_path` to every migration/seed it applies, so the apply step no
  longer depends on the full-bootstrap `ALTER DATABASE` having run (it is
  skipped under `SESAME_IDAM_APPLY_MIGRATIONS_ONLY=1`).
  (Follow-up for the migrator: emit fully-qualified names.)
- The container superuser bypasses RLS; the RLS contract migrations still
  apply cleanly and provide the `rls_set_*` functions the code calls.
- Mailpit-dependent email tests additionally need
  `mailpit.data.svc.cluster.local` resolvable (in-cluster: native; build
  hosts: resolver/hosts entry) — see `tests/bdd/email_round_trip.rs`.
- 2026-07-24: applied to `e2e-sesame-pg` on ms02 → full board 732/732.
