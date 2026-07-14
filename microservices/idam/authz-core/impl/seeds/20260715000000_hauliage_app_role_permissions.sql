-- Dev seed — hauliage RBAC permission catalog for principal_effective / JWT sx.permissions.
-- Apply after app_role_permissions migration and hauliage_demo_roles seed.

INSERT INTO sesame_idam.app_role_permissions (tenant_id, app_id, role_name, permission, created_at)
VALUES
    -- OWNER — org admin (shipper/transporter test personas)
    ('hauliage', 'frontend', 'OWNER', 'organization:read', NOW()),
    ('hauliage', 'frontend', 'OWNER', 'organization:write', NOW()),
    ('hauliage', 'frontend', 'OWNER', 'users:manage', NOW()),
    ('hauliage', 'frontend', 'OWNER', 'org:manage', NOW()),
    -- DISPATCHER
    ('hauliage', 'frontend', 'DISPATCHER', 'loads:read', NOW()),
    ('hauliage', 'frontend', 'DISPATCHER', 'loads:write', NOW()),
    ('hauliage', 'frontend', 'DISPATCHER', 'fleet:read', NOW()),
    -- FLEET_MANAGER
    ('hauliage', 'frontend', 'FLEET_MANAGER', 'fleet:read', NOW()),
    ('hauliage', 'frontend', 'FLEET_MANAGER', 'fleet:write', NOW()),
    -- DRIVER
    ('hauliage', 'frontend', 'DRIVER', 'loads:read', NOW()),
    -- VIEWER
    ('hauliage', 'frontend', 'VIEWER', 'organization:read', NOW())
ON CONFLICT ON CONSTRAINT app_role_permissions_unique DO NOTHING;
