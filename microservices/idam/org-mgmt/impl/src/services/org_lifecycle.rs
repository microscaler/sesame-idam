//! Organization lifecycle — create org, membership, invites (Sesame source of truth).
//!
//! New/edited data access uses the Lifeguard ORM (Entity/Column/Record). The
//! invite/accept/list functions below still use raw `sea_query` SQL — legacy,
//! to be migrated to the ORM opportunistically.

use chrono::{Duration, Utc};
use lifeguard::active_model::ActiveModelTrait;
use lifeguard::{ColumnTrait, LifeExecutor, LifeModelTrait};
use uuid::Uuid;

use crate::models::org::{Column as OrgColumn, Entity as OrgEntity, OrgRecord};
use crate::models::org_invite::{Column as InviteColumn, Entity as InviteEntity, OrgInviteRecord};
use crate::models::org_membership::{
    Column as MembershipColumn, Entity as MembershipEntity, OrgMembershipModel, OrgMembershipRecord,
};

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

impl std::fmt::Display for OrgLifecycleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Db(message) => write!(formatter, "database error: {message}"),
            Self::InvalidId(message) => write!(formatter, "invalid id: {message}"),
            Self::AlreadyHasOrganization => formatter.write_str("user already has an organization"),
            Self::NotFound => formatter.write_str("organization or invitation not found"),
            Self::Forbidden => formatter.write_str("organization access forbidden"),
            Self::InviteExpired => formatter.write_str("invitation expired"),
            Self::EmailMismatch => formatter.write_str("invitation email does not match user"),
        }
    }
}

impl std::error::Error for OrgLifecycleError {}

#[derive(Debug)]
pub struct OrganizationSummary {
    pub id: Uuid,
    pub name: String,
    pub tenant_id: String,
}

#[derive(Debug)]
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
    pub metadata: Option<serde_json::Value>,
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

    // Org must exist within the caller's tenant. Scoping the lookup by tenant_id
    // enforces cross-tenant isolation and avoids leaking org existence: a missing
    // or wrong-tenant org is reported as Forbidden, identical to a non-member.
    let org = OrgEntity::find()
        .filter(OrgColumn::Id.eq(org_uuid))
        .filter(OrgColumn::TenantId.eq(tenant_id.to_string()))
        .find_one(exec)
        .map_err(|e| OrgLifecycleError::Db(e.to_string()))?
        .ok_or(OrgLifecycleError::Forbidden)?;

    // Caller must be a member of the org.
    let membership = MembershipEntity::find()
        .filter(MembershipColumn::OrgId.eq(org_uuid))
        .filter(MembershipColumn::UserId.eq(user_uuid))
        .find_one(exec)
        .map_err(|e| OrgLifecycleError::Db(e.to_string()))?;
    if membership.is_none() {
        return Err(OrgLifecycleError::Forbidden);
    }

    Ok(OrgDetail {
        id: org.id,
        name: org.name,
        tenant_id: org.tenant_id,
        status: org.status,
        metadata: org.metadata,
        created_at: org.created_at,
        updated_at: org.updated_at,
    })
}

/// Create org + owner membership. Caller becomes active owner. `metadata` is
/// opaque tenant product metadata (e.g. Hauliage persona) persisted verbatim.
pub fn create_organization<E: LifeExecutor>(
    exec: &E,
    tenant_id: &str,
    user_id: &str,
    name: &str,
    metadata: Option<&serde_json::Value>,
) -> Result<OrganizationSummary, OrgLifecycleError> {
    let user_uuid =
        Uuid::parse_str(user_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;

    if user_has_active_org(exec, user_uuid, tenant_id)? {
        return Err(OrgLifecycleError::AlreadyHasOrganization);
    }

    let org_id = Uuid::new_v4();
    let now = Utc::now();

    let mut org_rec = OrgRecord::new();
    org_rec
        .set_id(org_id)
        .set_name(name.to_string())
        .set_tenant_id(tenant_id.to_string())
        .set_status("active".to_string())
        .set_metadata(metadata.cloned())
        .set_created_at(now)
        .set_updated_at(now);
    org_rec
        .insert(exec)
        .map_err(|e| OrgLifecycleError::Db(format!("organizations: {e}")))?;

    let mut membership_rec = OrgMembershipRecord::new();
    membership_rec
        .set_id(Uuid::new_v4())
        .set_org_id(org_id)
        .set_user_id(user_uuid)
        .set_role("owner".to_string())
        .set_status("active".to_string())
        .set_created_at(now)
        .set_updated_at(now);
    membership_rec
        .insert(exec)
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

    let mut memberships = MembershipEntity::find()
        .filter(MembershipColumn::UserId.eq(user_uuid))
        .all(exec)
        .map_err(|e| OrgLifecycleError::Db(e.to_string()))?;
    // ORDER BY created_at ASC (sorted client-side to keep the query single-column).
    memberships.sort_by_key(|membership| membership.created_at);

    let mut out = Vec::new();
    for m in memberships {
        // Only include memberships whose org belongs to the caller's tenant.
        if let Some(org) = OrgEntity::find()
            .filter(OrgColumn::Id.eq(m.org_id))
            .filter(OrgColumn::TenantId.eq(tenant_id.to_string()))
            .find_one(exec)
            .map_err(|e| OrgLifecycleError::Db(e.to_string()))?
        {
            out.push(MembershipSummary {
                org_id: m.org_id,
                org_name: org.name,
                role: m.role,
                status: m.status,
            });
        }
    }
    Ok(out)
}

#[derive(Debug, Clone)]
pub struct InviteCreated {
    pub invite_id: Uuid,
    pub invite_token: String,
}

pub fn invite_by_email<E: LifeExecutor>(
    exec: &E,
    tenant_id: &str,
    org_id: &str,
    email: &str,
    role: &str,
) -> Result<InviteCreated, OrgLifecycleError> {
    let org_uuid =
        Uuid::parse_str(org_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;
    ensure_org_tenant(exec, org_uuid, tenant_id)?;

    let invite_id = Uuid::new_v4();
    let token = Uuid::new_v4().to_string();
    let now = Utc::now();
    let expires = now + Duration::days(7);

    let mut invite_rec = OrgInviteRecord::new();
    invite_rec
        .set_id(invite_id)
        .set_org_id(org_uuid)
        .set_email(email.to_lowercase())
        .set_role(role.to_string())
        .set_token(token.clone())
        .set_expires_at(expires)
        .set_created_at(now)
        .set_accepted_at(None);
    invite_rec
        .insert(exec)
        .map_err(|e| OrgLifecycleError::Db(format!("org_invites: {e}")))?;

    tracing::info!(
        email = %email,
        org_id = %org_uuid,
        invite_id = %invite_id,
        "org invite created"
    );

    Ok(InviteCreated {
        invite_id,
        invite_token: token,
    })
}

#[derive(Debug)]
pub struct InvitePreview {
    pub organization_name: String,
    pub valid: bool,
    pub expired: bool,
}

/// Public-ish preview of an invitation by token (org name + validity) for the
/// onboarding UX. Tenant-scoped; the token itself is the capability. `NotFound`
/// when the token is unknown or its org is outside the tenant.
pub fn preview_invitation<E: LifeExecutor>(
    exec: &E,
    tenant_id: &str,
    token: &str,
) -> Result<InvitePreview, OrgLifecycleError> {
    let invite = InviteEntity::find()
        .filter(InviteColumn::Token.eq(token.to_string()))
        .find_one(exec)
        .map_err(|e| OrgLifecycleError::Db(e.to_string()))?
        .ok_or(OrgLifecycleError::NotFound)?;

    let org = OrgEntity::find()
        .filter(OrgColumn::Id.eq(invite.org_id))
        .filter(OrgColumn::TenantId.eq(tenant_id.to_string()))
        .find_one(exec)
        .map_err(|e| OrgLifecycleError::Db(e.to_string()))?
        .ok_or(OrgLifecycleError::NotFound)?;

    let expired = invite.expires_at < Utc::now();
    let valid = invite.accepted_at.is_none() && !expired;

    Ok(InvitePreview {
        organization_name: org.name,
        valid,
        expired,
    })
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

    // Find the pending (unaccepted) invite by token.
    let invite = InviteEntity::find()
        .filter(InviteColumn::Token.eq(token.to_string()))
        .find_one(exec)
        .map_err(|e| OrgLifecycleError::Db(e.to_string()))?
        .filter(|i| i.accepted_at.is_none())
        .ok_or(OrgLifecycleError::NotFound)?;

    // The invite's org must belong to the caller's tenant.
    let org = OrgEntity::find()
        .filter(OrgColumn::Id.eq(invite.org_id))
        .filter(OrgColumn::TenantId.eq(tenant_id.to_string()))
        .find_one(exec)
        .map_err(|e| OrgLifecycleError::Db(e.to_string()))?
        .ok_or(OrgLifecycleError::NotFound)?;

    if invite.expires_at < Utc::now() {
        return Err(OrgLifecycleError::InviteExpired);
    }

    if invite.email.trim().to_lowercase() != user_email.trim().to_lowercase() {
        return Err(OrgLifecycleError::EmailMismatch);
    }

    let now = Utc::now();

    let mut membership_rec = OrgMembershipRecord::new();
    membership_rec
        .set_id(Uuid::new_v4())
        .set_org_id(invite.org_id)
        .set_user_id(user_uuid)
        .set_role(invite.role.clone())
        .set_status("active".to_string())
        .set_created_at(now)
        .set_updated_at(now);
    membership_rec
        .insert(exec)
        .map_err(|e| OrgLifecycleError::Db(format!("org_memberships: {e}")))?;

    // Mark the invite accepted (full-record update by primary key).
    let mut invite_rec = OrgInviteRecord::new();
    invite_rec
        .set_id(invite.id)
        .set_org_id(invite.org_id)
        .set_email(invite.email.clone())
        .set_role(invite.role.clone())
        .set_token(invite.token.clone())
        .set_expires_at(invite.expires_at)
        .set_created_at(invite.created_at)
        .set_accepted_at(Some(now));
    invite_rec
        .update(exec)
        .map_err(|e| OrgLifecycleError::Db(format!("org_invites accept: {e}")))?;

    Ok(OrganizationSummary {
        id: invite.org_id,
        name: org.name,
        tenant_id: tenant_id.to_string(),
    })
}

fn user_has_active_org<E: LifeExecutor>(
    exec: &E,
    user_id: Uuid,
    tenant_id: &str,
) -> Result<bool, OrgLifecycleError> {
    let memberships = MembershipEntity::find()
        .filter(MembershipColumn::UserId.eq(user_id))
        .filter(MembershipColumn::Status.eq("active".to_string()))
        .all(exec)
        .map_err(|e| OrgLifecycleError::Db(e.to_string()))?;

    for m in memberships {
        let in_tenant = OrgEntity::find()
            .filter(OrgColumn::Id.eq(m.org_id))
            .filter(OrgColumn::TenantId.eq(tenant_id.to_string()))
            .find_one(exec)
            .map_err(|e| OrgLifecycleError::Db(e.to_string()))?
            .is_some();
        if in_tenant {
            return Ok(true);
        }
    }
    Ok(false)
}

fn ensure_org_tenant<E: LifeExecutor>(
    exec: &E,
    org_id: Uuid,
    tenant_id: &str,
) -> Result<(), OrgLifecycleError> {
    OrgEntity::find()
        .filter(OrgColumn::Id.eq(org_id))
        .filter(OrgColumn::TenantId.eq(tenant_id.to_string()))
        .find_one(exec)
        .map_err(|e| OrgLifecycleError::Db(e.to_string()))?
        .ok_or(OrgLifecycleError::NotFound)?;
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

    // Idempotent (replaces the raw INSERT ... ON CONFLICT DO NOTHING): skip when
    // the user is already a member of the org.
    let already_member = MembershipEntity::find()
        .filter(MembershipColumn::OrgId.eq(org_uuid))
        .filter(MembershipColumn::UserId.eq(user_uuid))
        .find_one(exec)
        .map_err(|e| OrgLifecycleError::Db(e.to_string()))?
        .is_some();
    if already_member {
        return Ok(());
    }

    let now = Utc::now();

    let mut membership_rec = OrgMembershipRecord::new();
    membership_rec
        .set_id(Uuid::new_v4())
        .set_org_id(org_uuid)
        .set_user_id(user_uuid)
        .set_role(role.to_string())
        .set_status("active".to_string())
        .set_created_at(now)
        .set_updated_at(now);
    membership_rec
        .insert(exec)
        .map_err(|e| OrgLifecycleError::Db(format!("org_memberships: {e}")))?;

    Ok(())
}

#[derive(Debug)]
pub struct OrgMemberSummary {
    pub user_id: Uuid,
    pub email: String,
    pub role: String,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug)]
pub struct PaginatedMembers {
    pub items: Vec<OrgMemberSummary>,
    pub total: i32,
    pub page: i32,
    pub page_size: i32,
}

fn membership_for_user<E: LifeExecutor>(
    exec: &E,
    org_id: Uuid,
    user_id: Uuid,
) -> Result<Option<OrgMembershipModel>, OrgLifecycleError> {
    MembershipEntity::find()
        .filter(MembershipColumn::OrgId.eq(org_id))
        .filter(MembershipColumn::UserId.eq(user_id))
        .filter(MembershipColumn::Status.eq("active".to_string()))
        .find_one(exec)
        .map_err(|e| OrgLifecycleError::Db(e.to_string()))
}

fn require_org_member<E: LifeExecutor>(
    exec: &E,
    tenant_id: &str,
    org_id: Uuid,
    user_id: Uuid,
) -> Result<(), OrgLifecycleError> {
    ensure_org_tenant(exec, org_id, tenant_id)?;
    let membership = membership_for_user(exec, org_id, user_id)?;
    if membership.is_none() {
        return Err(OrgLifecycleError::Forbidden);
    }
    Ok(())
}

fn require_org_admin<E: LifeExecutor>(
    exec: &E,
    tenant_id: &str,
    org_id: Uuid,
    user_id: Uuid,
) -> Result<(), OrgLifecycleError> {
    ensure_org_tenant(exec, org_id, tenant_id)?;
    let Some(membership) = membership_for_user(exec, org_id, user_id)? else {
        return Err(OrgLifecycleError::Forbidden);
    };
    let role = membership.role.to_ascii_lowercase();
    if role != "owner" && role != "admin" {
        return Err(OrgLifecycleError::Forbidden);
    }
    Ok(())
}

/// List active members of an organization. Caller must be a member.
pub fn list_org_members<E: LifeExecutor>(
    exec: &E,
    tenant_id: &str,
    org_id: &str,
    caller_user_id: &str,
    role_filter: Option<&str>,
    page_number: i32,
    page_size: i32,
) -> Result<PaginatedMembers, OrgLifecycleError> {
    let org_uuid =
        Uuid::parse_str(org_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;
    let caller_uuid =
        Uuid::parse_str(caller_user_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;
    require_org_member(exec, tenant_id, org_uuid, caller_uuid)?;

    let page_size = page_size.clamp(1, 100);
    let page_number = page_number.max(0);
    let offset = i64::from(page_number) * i64::from(page_size);

    let mut memberships = MembershipEntity::find()
        .filter(MembershipColumn::OrgId.eq(org_uuid))
        .filter(MembershipColumn::Status.eq("active".to_string()))
        .all(exec)
        .map_err(|e| OrgLifecycleError::Db(e.to_string()))?;
    memberships.sort_by_key(|m| m.created_at);

    if let Some(role) = role_filter {
        memberships.retain(|m| m.role == role);
    }

    let total = i32::try_from(memberships.len()).unwrap_or(i32::MAX);
    let page_items: Vec<_> = memberships
        .into_iter()
        .skip(usize::try_from(offset).unwrap_or(usize::MAX))
        .take(usize::try_from(page_size).unwrap_or(0))
        .collect();

    let mut items = Vec::with_capacity(page_items.len());
    for membership in page_items {
        let email = lifeguard::query_value::<String, _>(
            exec,
            "SELECT email FROM sesame_idam.users WHERE id = $1",
            &[&membership.user_id],
        )
        .ok()
        .unwrap_or_else(|| format!("user-{}", membership.user_id));
        items.push(OrgMemberSummary {
            user_id: membership.user_id,
            email,
            role: membership.role.clone(),
            created_at: membership.created_at,
        });
    }

    Ok(PaginatedMembers {
        items,
        total,
        page: page_number,
        page_size,
    })
}

/// Change a member's primary role. Caller must be org owner/admin.
pub fn change_member_role<E: LifeExecutor>(
    exec: &E,
    tenant_id: &str,
    org_id: &str,
    caller_user_id: &str,
    target_user_id: &str,
    primary_role: &str,
) -> Result<(), OrgLifecycleError> {
    let org_uuid =
        Uuid::parse_str(org_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;
    let caller_uuid =
        Uuid::parse_str(caller_user_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;
    let target_uuid =
        Uuid::parse_str(target_user_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;
    require_org_admin(exec, tenant_id, org_uuid, caller_uuid)?;

    let role = primary_role.trim();
    if role.is_empty() {
        return Err(OrgLifecycleError::InvalidId(
            "primary_role is required".to_string(),
        ));
    }

    let Some(membership) = MembershipEntity::find()
        .filter(MembershipColumn::OrgId.eq(org_uuid))
        .filter(MembershipColumn::UserId.eq(target_uuid))
        .filter(MembershipColumn::Status.eq("active".to_string()))
        .find_one(exec)
        .map_err(|e| OrgLifecycleError::Db(e.to_string()))?
    else {
        return Err(OrgLifecycleError::NotFound);
    };

    let now = Utc::now();
    let mut rec = OrgMembershipRecord::new();
    rec.set_id(membership.id)
        .set_org_id(membership.org_id)
        .set_user_id(membership.user_id)
        .set_role(role.to_string())
        .set_status(membership.status.clone())
        .set_created_at(membership.created_at)
        .set_updated_at(now);
    rec.update(exec)
        .map_err(|e| OrgLifecycleError::Db(format!("org_memberships update: {e}")))?;
    Ok(())
}

/// Remove a member from an organization. Caller must be org owner/admin.
pub fn remove_member<E: LifeExecutor>(
    exec: &E,
    tenant_id: &str,
    org_id: &str,
    caller_user_id: &str,
    target_user_id: &str,
) -> Result<(), OrgLifecycleError> {
    let org_uuid =
        Uuid::parse_str(org_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;
    let caller_uuid =
        Uuid::parse_str(caller_user_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;
    let target_uuid =
        Uuid::parse_str(target_user_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;
    require_org_admin(exec, tenant_id, org_uuid, caller_uuid)?;

    if caller_uuid == target_uuid {
        return Err(OrgLifecycleError::Forbidden);
    }

    let Some(membership) = MembershipEntity::find()
        .filter(MembershipColumn::OrgId.eq(org_uuid))
        .filter(MembershipColumn::UserId.eq(target_uuid))
        .find_one(exec)
        .map_err(|e| OrgLifecycleError::Db(e.to_string()))?
    else {
        return Err(OrgLifecycleError::NotFound);
    };

    lifeguard::LifeExecutor::execute_values(
        exec,
        "DELETE FROM sesame_idam.org_memberships WHERE id = $1",
        &sea_query::Values(vec![membership.id.into()]),
    )
    .map_err(|e| OrgLifecycleError::Db(format!("org_memberships delete: {e}")))?;
    Ok(())
}

/// Revoke a pending invitation. Caller must be org owner/admin.
pub fn revoke_invite<E: LifeExecutor>(
    exec: &E,
    tenant_id: &str,
    org_id: &str,
    caller_user_id: &str,
    invite_id: &str,
) -> Result<(), OrgLifecycleError> {
    let org_uuid =
        Uuid::parse_str(org_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;
    let caller_uuid =
        Uuid::parse_str(caller_user_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;
    let invite_uuid =
        Uuid::parse_str(invite_id).map_err(|e| OrgLifecycleError::InvalidId(e.to_string()))?;
    require_org_admin(exec, tenant_id, org_uuid, caller_uuid)?;

    let invite = InviteEntity::find()
        .filter(InviteColumn::Id.eq(invite_uuid))
        .filter(InviteColumn::OrgId.eq(org_uuid))
        .find_one(exec)
        .map_err(|e| OrgLifecycleError::Db(e.to_string()))?
        .ok_or(OrgLifecycleError::NotFound)?;

    if invite.accepted_at.is_some() {
        return Err(OrgLifecycleError::NotFound);
    }

    lifeguard::LifeExecutor::execute_values(
        exec,
        "DELETE FROM sesame_idam.org_invites WHERE id = $1",
        &sea_query::Values(vec![invite.id.into()]),
    )
    .map_err(|e| OrgLifecycleError::Db(format!("org_invites delete: {e}")))?;
    Ok(())
}
