//! Principal role/attribute resolution for `/authz/principals/effective`.
//!
//! Stateless service (hauliage pattern): methods are generic over
//! `E: LifeExecutor`; the executor comes from `sesame_idam_database::db()`
//! at the controller edge.

use lifeguard::{ColumnTrait, LifeError, LifeExecutor, LifeModelTrait};
use uuid::Uuid;

use crate::models::principal_attribute::{
    Column as AttrColumn, Entity as AttrEntity, PrincipalAttributeModel,
};
use crate::models::role_assignment::{
    Column as RoleColumn, Entity as RoleEntity, RoleAssignmentModel,
};

pub struct PrincipalService;

impl PrincipalService {
    /// All role assignments for a principal within a tenant.
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] on query failure.
    pub fn role_assignments<E: LifeExecutor>(
        tenant_id: &str,
        principal_id: Uuid,
        exec: &E,
    ) -> Result<Vec<RoleAssignmentModel>, LifeError> {
        RoleEntity::find()
            .filter(RoleColumn::TenantId.eq(tenant_id.to_string()))
            .filter(RoleColumn::PrincipalId.eq(principal_id))
            .all(exec)
    }

    /// All custom attributes for a principal within a tenant.
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] on query failure.
    pub fn attributes<E: LifeExecutor>(
        tenant_id: &str,
        principal_id: Uuid,
        exec: &E,
    ) -> Result<Vec<PrincipalAttributeModel>, LifeError> {
        AttrEntity::find()
            .filter(AttrColumn::TenantId.eq(tenant_id.to_string()))
            .filter(AttrColumn::PrincipalId.eq(principal_id))
            .all(exec)
    }
}
