//! Organization lifecycle — create org, membership, invites (Sesame source of truth).

use chrono::{Duration, Utc};
use lifeguard::LifeExecutor;
use uuid::Uuid;

#[derive(Debug)]
pub enum OrgLifecycleError {
    Db(String),
    InvalidId(String),
    AlreadyHasOrganization,
    NotFound,
    Forbidden,
    InviteExpired,
    EmailMismatch,
}

pub struct OrganizationSummary {
    pub id: Uuid,
    pub name: String,
    pub tenant_id: String,
}

pub struct MembershipSummary {
    pub org_id: Uuid,
    pub org_name: String,
    pub role: String,
    pub status: String,
}

#[derive(Debug)]
pub struct OrgDetail {
    pub id: Uuid,
    pub name: String,
    pub tenant_id: String,
    pub status: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

/// Fetch org metadata. Caller must be a member of the org within the tenant.
///
/// Returns `Forbidden` when the user is not a member (indistinguishable from a
/// non-existent org across tenants — avoids leaking org existence), `NotFound`
/// when the org row is absent within the tenant.
pub fn get_organization<E: LifeExecutor>(
    exec: &E,
    tenant_id: &str,
    org_id: &str,
    user_id: &str,
) -> Result<OrgDetail, OrgLifecycleError> {
    let org_uuid =
        Uuid::parse_str(org_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;
    let user_uuid =
        Uuid::parse_str(user_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;

    // Membership check — also enforces tenant scoping via the org join.
    let is_member = exec
        .query_one_values(
            "SELECT 1 FROM sesame_idam.org_memberships om
             INNER JOIN sesame_idam.organizations o ON o.id = om.org_id
             WHERE om.org_id = $1 AND om.user_id = $2 AND o.tenant_id = $3
             LIMIT 1",
            &sea_query::Values(vec![org_uuid.into(), user_uuid.into(), tenant_id.into()]),
        )
        .is_ok();
    if !is_member {
        return Err(OrgLifecycleError::Forbidden);
    }

    let row = exec
        .query_one_values(
            "SELECT o.id::text, o.name, o.tenant_id, o.status, o.created_at, o.updated_at
             FROM sesame_idam.organizations o
             WHERE o.id = $1 AND o.tenant_id = $2",
            &sea_query::Values(vec![org_uuid.into(), tenant_id.into()]),
        )
        .map_err(|_| OrgLifecycleError::NotFound)?;

    let id: String = row.get(0);
    Ok(OrgDetail {
        id: Uuid::parse_str(&id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?,
        name: row.get(1),
        tenant_id: row.get(2),
        status: row.get(3),
        created_at: row.get(4),
        updated_at: row.get(5),
    })
}

/// Create org + owner membership. Caller becomes active owner.
pub fn create_organization<E: LifeExecutor>(
    exec: &E,
    tenant_id: &str,
    user_id: &str,
    name: &str,
) -> Result<OrganizationSummary, OrgLifecycleError> {
    let user_uuid =
        Uuid::parse_str(user_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;

    if user_has_active_org(exec, user_uuid, tenant_id)? {
        return Err(OrgLifecycleError::AlreadyHasOrganization);
    }

    let org_id = Uuid::new_v4();
    let membership_id = Uuid::new_v4();
    let now = Utc::now();

    exec.execute_values(
        "INSERT INTO sesame_idam.organizations (id, name, tenant_id, status, created_at, updated_at)
         VALUES ($1, $2, $3, 'active', $4, $4)",
        &sea_query::Values(vec![
            org_id.into(),
            name.into(),
            tenant_id.into(),
            now.into(),
        ]),
    )
    .map_err(|e| OrgLifecycleError::Db(format!("organizations: {e}")))?;

    exec.execute_values(
        "INSERT INTO sesame_idam.org_memberships (id, org_id, user_id, role, status, created_at, updated_at)
         VALUES ($1, $2, $3, 'owner', 'active', $4, $4)",
        &sea_query::Values(vec![
            membership_id.into(),
            org_id.into(),
            user_uuid.into(),
            now.into(),
        ]),
    )
    .map_err(|e| OrgLifecycleError::Db(format!("org_memberships: {e}")))?;

    Ok(OrganizationSummary {
        id: org_id,
        name: name.to_string(),
        tenant_id: tenant_id.to_string(),
    })
}

pub fn list_memberships<E: LifeExecutor>(
    exec: &E,
    tenant_id: &str,
    user_id: &str,
) -> Result<Vec<MembershipSummary>, OrgLifecycleError> {
    let user_uuid =
        Uuid::parse_str(user_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;

    let rows = exec
        .query_all_values(
            "SELECT o.id::text, o.name, om.role, om.status
             FROM sesame_idam.org_memberships om
             INNER JOIN sesame_idam.organizations o ON o.id = om.org_id
             WHERE om.user_id = $1 AND o.tenant_id = $2
             ORDER BY om.created_at ASC",
            &sea_query::Values(vec![user_uuid.into(), tenant_id.into()]),
        )
        .map_err(|e| OrgLifecycleError::Db(e.to_string()))?;

    let mut out = Vec::new();
    for row in rows {
        let org_id: String = row.get(0);
        let org_name: String = row.get(1);
        let role: String = row.get(2);
        let status: String = row.get(3);
        out.push(MembershipSummary {
            org_id: Uuid::parse_str(&org_id)
                .map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?,
            org_name,
            role,
            status,
        });
    }
    Ok(out)
}

pub fn invite_by_email<E: LifeExecutor>(
    exec: &E,
    tenant_id: &str,
    org_id: &str,
    email: &str,
    role: &str,
) -> Result<Uuid, OrgLifecycleError> {
    let org_uuid =
        Uuid::parse_str(org_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;
    ensure_org_tenant(exec, org_uuid, tenant_id)?;

    let invite_id = Uuid::new_v4();
    let token = Uuid::new_v4().to_string();
    let now = Utc::now();
    let expires = now + Duration::days(7);

    exec.execute_values(
        "INSERT INTO sesame_idam.org_invites (id, org_id, email, role, token, expires_at, created_at, accepted_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, NULL)",
        &sea_query::Values(vec![
            invite_id.into(),
            org_uuid.into(),
            email.to_lowercase().into(),
            role.into(),
            token.clone().into(),
            expires.into(),
            now.into(),
        ]),
    )
    .map_err(|e| OrgLifecycleError::Db(format!("org_invites: {e}")))?;

    tracing::info!(
        email = %email,
        org_id = %org_uuid,
        token = %token,
        "org invite created (dev — wire email delivery in production)"
    );

    Ok(invite_id)
}

pub fn accept_invitation<E: LifeExecutor>(
    exec: &E,
    tenant_id: &str,
    user_id: &str,
    user_email: &str,
    token: &str,
) -> Result<OrganizationSummary, OrgLifecycleError> {
    let user_uuid =
        Uuid::parse_str(user_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;

    if user_has_active_org(exec, user_uuid, tenant_id)? {
        return Err(OrgLifecycleError::AlreadyHasOrganization);
    }

    let row = exec
        .query_one_values(
            "SELECT i.id::text, i.org_id::text, i.email, i.role, i.expires_at, o.name, o.tenant_id
             FROM sesame_idam.org_invites i
             INNER JOIN sesame_idam.organizations o ON o.id = i.org_id
             WHERE i.token = $1 AND i.accepted_at IS NULL AND o.tenant_id = $2",
            &sea_query::Values(vec![token.into(), tenant_id.into()]),
        )
        .map_err(|_| OrgLifecycleError::NotFound)?;

    let invite_id: String = row.get(0);
    let org_id: String = row.get(1);
    let invite_email: String = row.get(2);
    let role: String = row.get(3);
    let expires_at: chrono::DateTime<Utc> = row.get(4);
    let org_name: String = row.get(5);

    if expires_at < Utc::now() {
        return Err(OrgLifecycleError::InviteExpired);
    }

    if invite_email.trim().to_lowercase() != user_email.trim().to_lowercase() {
        return Err(OrgLifecycleError::EmailMismatch);
    }

    let org_uuid =
        Uuid::parse_str(&org_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;
    let invite_uuid =
        Uuid::parse_str(&invite_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;
    let now = Utc::now();
    let membership_id = Uuid::new_v4();

    exec.execute_values(
        "INSERT INTO sesame_idam.org_memberships (id, org_id, user_id, role, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'active', $5, $5)",
        &sea_query::Values(vec![
            membership_id.into(),
            org_uuid.into(),
            user_uuid.into(),
            role.into(),
            now.into(),
        ]),
    )
    .map_err(|e| OrgLifecycleError::Db(format!("org_memberships: {e}")))?;

    exec.execute_values(
        "UPDATE sesame_idam.org_invites SET accepted_at = $1 WHERE id = $2",
        &sea_query::Values(vec![now.into(), invite_uuid.into()]),
    )
    .map_err(|e| OrgLifecycleError::Db(format!("org_invites accept: {e}")))?;

    Ok(OrganizationSummary {
        id: org_uuid,
        name: org_name,
        tenant_id: tenant_id.to_string(),
    })
}

fn user_has_active_org<E: LifeExecutor>(
    exec: &E,
    user_id: Uuid,
    tenant_id: &str,
) -> Result<bool, OrgLifecycleError> {
    let exists = exec
        .query_one_values(
            "SELECT 1 FROM sesame_idam.org_memberships om
             INNER JOIN sesame_idam.organizations o ON o.id = om.org_id
             WHERE om.user_id = $1 AND om.status = 'active' AND o.tenant_id = $2
             LIMIT 1",
            &sea_query::Values(vec![user_id.into(), tenant_id.into()]),
        )
        .is_ok();
    Ok(exists)
}

fn ensure_org_tenant<E: LifeExecutor>(
    exec: &E,
    org_id: Uuid,
    tenant_id: &str,
) -> Result<(), OrgLifecycleError> {
    exec.query_one_values(
        "SELECT 1 FROM sesame_idam.organizations WHERE id = $1 AND tenant_id = $2",
        &sea_query::Values(vec![org_id.into(), tenant_id.into()]),
    )
    .map_err(|_| OrgLifecycleError::NotFound)?;
    Ok(())
}

pub fn add_user_membership<E: LifeExecutor>(
    exec: &E,
    tenant_id: &str,
    org_id: &str,
    user_id: &str,
    role: &str,
) -> Result<(), OrgLifecycleError> {
    let org_uuid =
        Uuid::parse_str(org_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;
    let user_uuid =
        Uuid::parse_str(user_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;
    ensure_org_tenant(exec, org_uuid, tenant_id)?;

    let membership_id = Uuid::new_v4();
    let now = Utc::now();

    exec.execute_values(
        "INSERT INTO sesame_idam.org_memberships (id, org_id, user_id, role, status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'active', $5, $5)
         ON CONFLICT DO NOTHING",
        &sea_query::Values(vec![
            membership_id.into(),
            org_uuid.into(),
            user_uuid.into(),
            role.into(),
            now.into(),
        ]),
    )
    .map_err(|e| OrgLifecycleError::Db(format!("org_memberships: {e}")))?;

    Ok(())
}
