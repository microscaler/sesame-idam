-- Migration: one assignment of a role to a user per scope (tenant-wide, or one organisation).
--
-- authz-core's assign reads, then inserts; two grants of the same role at the same moment both inserted, and a
-- revoke then had duplicates to find. Duplicates are removed (the oldest kept) and the index makes a second insert
-- fail, which assign reports as "already held". Role names compare case-insensitively, as everywhere in Sesame.

DELETE FROM sesame_idam.role_assignments a
USING sesame_idam.role_assignments b
WHERE a.tenant_id = b.tenant_id
  AND a.principal_id = b.principal_id
  AND lower(a.role_name) = lower(b.role_name)
  AND a.resource_id IS NOT DISTINCT FROM b.resource_id
  AND (a.created_at, a.id) > (b.created_at, b.id);

CREATE UNIQUE INDEX IF NOT EXISTS role_assignments_one_per_scope
    ON sesame_idam.role_assignments (
        tenant_id,
        principal_id,
        lower(role_name),
        COALESCE(resource_id, '00000000-0000-0000-0000-000000000000'::uuid)
    );
