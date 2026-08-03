-- Dev seed — acme RBAC permission catalog for principal_effective / JWT sx.permissions.
-- Apply after app_role_permissions migration and acme_demo_roles seed.

INSERT INTO sesame_idam.app_role_permissions (tenant_id, app_id, role_name, permission, created_at)
VALUES
    -- OWNER — org admin (shipper/transporter test personas)
    ('acme', 'frontend', 'OWNER', 'organization:read', NOW()),
    ('acme', 'frontend', 'OWNER', 'organization:write', NOW()),
    ('acme', 'frontend', 'OWNER', 'users:manage', NOW()),
    ('acme', 'frontend', 'OWNER', 'org:manage', NOW()),
    -- DISPATCHER
    ('acme', 'frontend', 'DISPATCHER', 'loads:read', NOW()),
    ('acme', 'frontend', 'DISPATCHER', 'loads:write', NOW()),
    ('acme', 'frontend', 'DISPATCHER', 'fleet:read', NOW()),
    -- FLEET_MANAGER
    ('acme', 'frontend', 'FLEET_MANAGER', 'fleet:read', NOW()),
    ('acme', 'frontend', 'FLEET_MANAGER', 'fleet:write', NOW()),
    -- DRIVER
    ('acme', 'frontend', 'DRIVER', 'loads:read', NOW()),
    -- VIEWER
    ('acme', 'frontend', 'VIEWER', 'organization:read', NOW())
ON CONFLICT ON CONSTRAINT app_role_permissions_unique DO NOTHING;
