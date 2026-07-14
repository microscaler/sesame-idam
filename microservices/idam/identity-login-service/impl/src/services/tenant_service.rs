//! Platform tenant registry — provisioned tenants only (no magic slugs).

use chrono::Utc;
use lifeguard::active_model::ActiveModelTrait;
use lifeguard::{ColumnTrait, LifeError, LifeExecutor, LifeModelTrait};
use uuid::Uuid;

use crate::models::tenant::{Column, Entity, TenantModel, TenantRecord};

pub const STATUS_ACTIVE: &str = "active";
pub const STATUS_SUSPENDED: &str = "suspended";
pub const STATUS_PROVISIONING: &str = "provisioning";
pub const STATUS_DEPROVISIONED: &str = "deprovisioned";
pub const STATUS_FAILED: &str = "failed";

pub const PROVISIONING_PLATFORM: &str = "platform";
pub const PROVISIONING_SELF_SERVICE: &str = "self_service";

const RESERVED_SLUGS: &[&str] = &["admin", "api", "idam", "platform", "www", "sesame"];

/// Slug validation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlugValidationError {
    Empty,
    InvalidFormat,
    Reserved,
}

impl SlugValidationError {
    #[must_use]
    pub fn api_error(&self) -> &'static str {
        match self {
            Self::Empty | Self::InvalidFormat => "invalid_slug",
            Self::Reserved => "reserved_slug",
        }
    }
}

/// Why a tenant could not be used for auth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TenantGateError {
    Unknown,
    NotActive,
    Db(String),
}

impl TenantGateError {
    #[must_use]
    pub fn api_error(&self) -> &'static str {
        match self {
            Self::Unknown => "tenant_unknown",
            Self::NotActive => "tenant_not_active",
            Self::Db(_) => "internal_error",
        }
    }
}

pub struct TenantService;

impl TenantService {
    /// Validate tenant slug format and reserved list.
    pub fn validate_slug(slug: &str) -> Result<String, SlugValidationError> {
        let slug = slug.trim().to_ascii_lowercase();
        if slug.is_empty() {
            return Err(SlugValidationError::Empty);
        }
        if slug.len() < 3 || slug.len() > 63 {
            return Err(SlugValidationError::InvalidFormat);
        }
        if !slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            || slug.starts_with('-')
            || slug.ends_with('-')
        {
            return Err(SlugValidationError::InvalidFormat);
        }
        if RESERVED_SLUGS.contains(&slug.as_str()) {
            return Err(SlugValidationError::Reserved);
        }
        Ok(slug)
    }

    /// Serialize tenant model for platform API responses.
    #[must_use]
    pub fn to_json(tenant: &TenantModel) -> serde_json::Value {
        serde_json::json!({
            "id": tenant.id.to_string(),
            "slug": tenant.slug,
            "display_name": tenant.display_name,
            "status": tenant.status,
            "provisioning_mode": tenant.provisioning_mode,
            "created_at": tenant.created_at.to_rfc3339(),
            "updated_at": tenant.updated_at.to_rfc3339(),
        })
    }

    /// Lookup by slug (`X-Tenant-ID`).
    pub fn find_by_slug<E: LifeExecutor>(
        slug: &str,
        exec: &E,
    ) -> Result<Option<TenantModel>, LifeError> {
        Entity::find()
            .filter(Column::Slug.eq(slug.trim().to_string()))
            .find_one(exec)
    }

    /// Require a registered, active tenant before any auth operation.
    pub fn require_active<E: LifeExecutor>(
        slug: &str,
        exec: &E,
    ) -> Result<TenantModel, TenantGateError> {
        let slug = slug.trim();
        if slug.is_empty() {
            return Err(TenantGateError::Unknown);
        }
        match Self::find_by_slug(slug, exec) {
            Ok(Some(t)) if t.status == STATUS_ACTIVE => Ok(t),
            Ok(Some(_)) => Err(TenantGateError::NotActive),
            Ok(None) => Err(TenantGateError::Unknown),
            Err(e) => Err(TenantGateError::Db(e.to_string())),
        }
    }

    /// Mint a tenant (platform admin or self-service provisioning).
    pub fn create<E: LifeExecutor>(
        slug: &str,
        display_name: &str,
        provisioning_mode: &str,
        status: &str,
        exec: &E,
    ) -> Result<Uuid, LifeError> {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let mut record = TenantRecord::new();
        record
            .set_id(id)
            .set_slug(slug.trim().to_string())
            .set_display_name(display_name.trim().to_string())
            .set_status(status.to_string())
            .set_provisioning_mode(provisioning_mode.to_string())
            .set_created_at(now)
            .set_updated_at(now);
        record
            .insert(exec)
            .map_err(|e| LifeError::Other(e.to_string()))?;
        Ok(id)
    }

    /// Convenience: create active platform tenant.
    pub fn create_active_platform<E: LifeExecutor>(
        slug: &str,
        display_name: &str,
        exec: &E,
    ) -> Result<Uuid, LifeError> {
        Self::create(
            slug,
            display_name,
            PROVISIONING_PLATFORM,
            STATUS_ACTIVE,
            exec,
        )
    }

    /// Validated status transition for platform admin API.
    pub fn transition_status<E: LifeExecutor>(
        slug: &str,
        new_status: &str,
        exec: &E,
    ) -> Result<TenantModel, StatusTransitionError> {
        let tenant = Self::find_by_slug(slug, exec)
            .map_err(|e| StatusTransitionError::Db(e.to_string()))?
            .ok_or(StatusTransitionError::NotFound)?;

        if !is_allowed_transition(&tenant.status, new_status) {
            return Err(StatusTransitionError::InvalidTransition);
        }

        let now = Utc::now();
        let mut record = TenantRecord::new();
        record
            .set_id(tenant.id)
            .set_slug(tenant.slug.clone())
            .set_display_name(tenant.display_name.clone())
            .set_status(new_status.to_string())
            .set_provisioning_mode(tenant.provisioning_mode.clone())
            .set_created_at(tenant.created_at)
            .set_updated_at(now);
        record
            .update(exec)
            .map_err(|e| StatusTransitionError::Db(e.to_string()))?;

        Self::find_by_slug(slug, exec)
            .map_err(|e| StatusTransitionError::Db(e.to_string()))?
            .ok_or(StatusTransitionError::NotFound)
    }
}

/// Status transition failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusTransitionError {
    NotFound,
    InvalidTransition,
    Db(String),
}

impl StatusTransitionError {
    #[must_use]
    pub fn http_status(&self) -> u16 {
        match self {
            Self::NotFound => 404,
            Self::InvalidTransition => 409,
            Self::Db(_) => 500,
        }
    }

    #[must_use]
    pub fn api_error(&self) -> &'static str {
        match self {
            Self::NotFound => "tenant_not_found",
            Self::InvalidTransition => "invalid_status_transition",
            Self::Db(_) => "internal_error",
        }
    }
}

fn is_allowed_transition(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        (STATUS_ACTIVE, STATUS_SUSPENDED | STATUS_DEPROVISIONED)
            | (STATUS_SUSPENDED | STATUS_PROVISIONING, STATUS_ACTIVE)
            | (STATUS_PROVISIONING, STATUS_FAILED)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_slug_rejects_reserved() {
        assert_eq!(
            TenantService::validate_slug("admin"),
            Err(SlugValidationError::Reserved)
        );
    }

    #[test]
    fn validate_slug_accepts_hauliage() {
        assert_eq!(
            TenantService::validate_slug("hauliage"),
            Ok("hauliage".to_string())
        );
    }
}
