//! Resolve active organization membership for JWT `org_id` claim.

use lifeguard::LifeExecutor;
use uuid::Uuid;

/// Active org for a user within a tenant.
///
/// When `preferred_org_id` is set, returns it only if the user has an active membership.
/// Otherwise returns the earliest active membership for the tenant.
pub fn resolve_active_org_id<E: LifeExecutor>(
    exec: &E,
    user_id: &str,
    tenant_id: &str,
    preferred_org_id: Option<&str>,
) -> Option<Uuid> {
    let user_uuid = Uuid::parse_str(user_id).ok()?;

    if let Some(pref) = preferred_org_id {
        if let Ok(pref_uuid) = Uuid::parse_str(pref) {
            if membership_exists(exec, user_uuid, pref_uuid, tenant_id) {
                return Some(pref_uuid);
            }
        }
    }

    let row = exec
        .query_one_values(
            "SELECT om.org_id::text FROM sesame_idam.org_memberships om
             INNER JOIN sesame_idam.organizations o ON o.id = om.org_id
             WHERE om.user_id = $1 AND om.status = 'active' AND o.tenant_id = $2
             ORDER BY om.created_at ASC
             LIMIT 1",
            &sea_query::Values(vec![user_uuid.into(), tenant_id.into()]),
        )
        .ok()?;
    let org_str: String = row.get(0);
    Uuid::parse_str(&org_str).ok()
}

fn membership_exists<E: LifeExecutor>(
    exec: &E,
    user_id: Uuid,
    org_id: Uuid,
    tenant_id: &str,
) -> bool {
    exec.query_one_values(
        "SELECT 1 FROM sesame_idam.org_memberships om
         INNER JOIN sesame_idam.organizations o ON o.id = om.org_id
         WHERE om.user_id = $1 AND om.org_id = $2 AND om.status = 'active' AND o.tenant_id = $3",
        &sea_query::Values(vec![user_id.into(), org_id.into(), tenant_id.into()]),
    )
    .is_ok()
}
