//! Platform tenant registry — provisioned tenants only (no magic slugs).

use chrono::Utc;
use lifeguard::active_model::ActiveModelTrait;
use lifeguard::{ColumnTrait, LifeError, LifeExecutor, LifeModelTrait};
use uuid::Uuid;

use crate::models::tenant::{Column, Entity, TenantModel, TenantRecord};

pub const STATUS_ACTIVE: &str = "active";
pub const STATUS_SUSPENDED: &str = "suspended";
pub const STATUS_PROVISIONING: &str = "provisioning";

pub const PROVISIONING_PLATFORM: &str = "platform";
pub const PROVISIONING_SELF_SERVICE: &str = "self_service";

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
        exec: &E,
    ) -> Result<Uuid, LifeError> {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let mut record = TenantRecord::new();
        record
            .set_id(id)
            .set_slug(slug.trim().to_string())
            .set_display_name(display_name.trim().to_string())
            .set_status(STATUS_ACTIVE.to_string())
            .set_provisioning_mode(provisioning_mode.to_string())
            .set_created_at(now)
            .set_updated_at(now);
        record
            .insert(exec)
            .map_err(|e| LifeError::Other(e.to_string()))?;
        Ok(id)
    }
}
