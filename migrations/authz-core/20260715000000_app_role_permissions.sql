-- Tenant/application role → permission catalog for principal_effective (v1).
-- Distinct from org-mgmt org-scoped roles; maps authz role_assignments.role_name.

CREATE TABLE IF NOT EXISTS sesame_idam.app_role_permissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id VARCHAR(255) NOT NULL,
    app_id VARCHAR(255) NOT NULL,
    role_name VARCHAR(255) NOT NULL,
    permission VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    CONSTRAINT app_role_permissions_unique UNIQUE (tenant_id, app_id, role_name, permission)
);

CREATE INDEX IF NOT EXISTS idx_app_role_permissions_lookup
    ON sesame_idam.app_role_permissions (tenant_id, app_id, role_name);
