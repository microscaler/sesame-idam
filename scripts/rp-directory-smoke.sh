#!/bin/bash
# Smoke test for the rp_directory bridge (PRD-OPENGROUPWARE F3).
# Spins a throwaway postgres:16, applies the minimal migration chain,
# seeds a tenant/user/app-password, and verifies:
#   1. an RP role can read credentials through rp_directory views
#   2. the same role CANNOT read the base sesame_idam tables
set -uo pipefail

M="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/migrations"
C=rp-pg-smoke

docker rm -f $C >/dev/null 2>&1
docker run -d --name $C -e POSTGRES_PASSWORD=test -e POSTGRES_USER=root \
    -e POSTGRES_DB=sesame -p 15433:5432 postgres:16 >/dev/null
sleep 6

P() { docker exec -i $C psql -U root -d sesame -v ON_ERROR_STOP=1 -q; }

echo "CREATE SCHEMA sesame_idam; CREATE ROLE sesame_idam NOLOGIN;" | P || exit 1
P < "$M/rls/20260714180000_sesame_rls_contract_v1.sql" && echo contract-ok
P < "$M/identity-login-service/20260714102157_tenants.sql" && echo tenants-ok
P < "$M/identity-user-mgmt-service/20260705235433_users.sql" && echo users-ok
P < "$M/identity-user-mgmt-service/20260716200000_app_passwords.sql" && echo app-passwords-ok
P < "$M/rls/20260716200001_rp_directory.sql" && echo rp-directory-ok

P <<'SQL'
INSERT INTO sesame_idam.tenants (slug, display_name, status, provisioning_mode, created_at, updated_at)
VALUES ('acme', 'Acme', 'active', 'platform', now(), now());
INSERT INTO sesame_idam.users (email, password_hash, tenant_id, status, created_at, updated_at)
VALUES ('charles@acme.example', '$argon2id$v=19$m=19456,t=2,p=1$FAKE$FAKE', 'acme', 'active', now(), now());
INSERT INTO sesame_idam.app_passwords (tenant_id, user_id, label, secret_phc)
SELECT 'acme', id, 'thunderbird', '$argon2id$v=19$FAKE2$FAKE2' FROM sesame_idam.users;
-- Suspended-tenant user must NOT appear in the views.
INSERT INTO sesame_idam.tenants (slug, display_name, status, provisioning_mode, created_at, updated_at)
VALUES ('ghost', 'Ghost', 'suspended', 'platform', now(), now());
INSERT INTO sesame_idam.users (email, password_hash, tenant_id, status, created_at, updated_at)
VALUES ('boo@ghost.example', '$argon2id$v=19$X$X', 'ghost', 'active', now(), now());
CREATE ROLE rp_stalwart LOGIN PASSWORD 'rptest';
GRANT rp_directory_read TO rp_stalwart;
SQL

RP() { docker exec -e PGPASSWORD=rptest $C psql -U rp_stalwart -d sesame -t -A -c "$1"; }

echo "== RP reads primary credential (expect 1 row, argon2id prefix)"
RP "SELECT login || ' ' || left(secret_phc, 10) FROM rp_directory.users WHERE login = 'charles@acme.example'"
echo "== RP reads app passwords (expect 1)"
RP "SELECT count(*) FROM rp_directory.app_passwords"
echo "== suspended tenant invisible (expect 0)"
RP "SELECT count(*) FROM rp_directory.users WHERE login = 'boo@ghost.example'"
echo "== RP blocked from base table (expect permission denied)"
RP "SELECT count(*) FROM sesame_idam.users" 2>&1 | head -1
echo "== RP blocked from writing views (expect error)"
RP "DELETE FROM rp_directory.users" 2>&1 | head -1

docker rm -f $C >/dev/null
echo "smoke complete"
