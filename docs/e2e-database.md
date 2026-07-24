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

# 1. search_path FIRST — a few migrations use unqualified table names.
docker exec e2e-sesame-pg psql -U root -d sesame \
  -c "ALTER DATABASE sesame SET search_path TO sesame_idam, public;"

# 2. Migrations in FK-safe order (idempotent CREATE IF NOT EXISTS).
while IFS= read -r rel; do case "$rel" in \#*|"") continue;; esac
  docker exec -i e2e-sesame-pg psql -q -U root -d sesame -v ON_ERROR_STOP=1 \
    < "migrations/$rel"
done < migrations/apply_order.txt

# 3. Demo seeds (hauliage users/orgs/roles, platform tenants) in FK order.
while IFS= read -r rel; do case "$rel" in \#*|"") continue;; esac
  docker exec -i e2e-sesame-pg psql -q -U root -d sesame -v ON_ERROR_STOP=1 \
    < "microservices/idam/$rel"
done < microservices/idam/seed_order.txt
```

Then run the suite:

```bash
TEST_DB_PORT=15434 TEST_DB_USER=root TEST_DB_PASS=test TEST_DB_NAME=sesame \
  cargo nextest run
```

Notes:

- Step 1 is load-bearing: `org-mgmt/20260516192347_permissions.sql`,
  `api-keys/20260516192347_api_keys.sql`, and
  `org-mgmt/20260713065042_organizations.sql` reference tables without the
  `sesame_idam.` prefix and fail without the database-level `search_path`.
  (Follow-up for the migrator: emit fully-qualified names.)
- The container superuser bypasses RLS; the RLS contract migrations still
  apply cleanly and provide the `rls_set_*` functions the code calls.
- Mailpit-dependent email tests additionally need
  `mailpit.data.svc.cluster.local` resolvable (in-cluster: native; build
  hosts: resolver/hosts entry) — see `tests/bdd/email_round_trip.rs`.
- 2026-07-24: applied to `e2e-sesame-pg` on ms02 → full board 732/732.
