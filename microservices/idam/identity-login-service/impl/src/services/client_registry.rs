use lifeguard::{ColumnTrait, LifeExecutor, LifeModelTrait};

use crate::models::relying_party_client::{Column, Entity};
use crate::services::tenant_service::TenantService;

pub const STATUS_ACTIVE: &str = "active";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientBinding {
    pub client_id: String,
    pub tenant_id: String,
    pub portal: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientRegistryError {
    Unknown,
    NotActive,
    Db(String),
}

fn binding_from_statuses(
    client_id: &str,
    tenant_id: &str,
    portal: &str,
    client_status: &str,
    tenant_status: &str,
) -> Result<ClientBinding, ClientRegistryError> {
    if client_status != STATUS_ACTIVE {
        return Err(ClientRegistryError::Unknown);
    }
    if tenant_status != STATUS_ACTIVE {
        return Err(ClientRegistryError::NotActive);
    }
    Ok(ClientBinding {
        client_id: client_id.to_string(),
        tenant_id: tenant_id.to_string(),
        portal: portal.to_string(),
    })
}

pub struct ClientRegistry;

impl ClientRegistry {
    /// Resolve an active registered client into trusted tenant/application context.
    pub fn resolve_active<E: LifeExecutor>(
        client_id: &str,
        exec: &E,
    ) -> Result<ClientBinding, ClientRegistryError> {
        let client_id = client_id.trim();
        if client_id.is_empty() {
            return Err(ClientRegistryError::Unknown);
        }

        let client = Entity::find()
            .filter(Column::ClientId.eq(client_id.to_string()))
            .find_one(exec)
            .map_err(|error| ClientRegistryError::Db(error.to_string()))?
            .ok_or(ClientRegistryError::Unknown)?;

        let tenant = TenantService::find_by_slug(&client.tenant_slug, exec)
            .map_err(|error| ClientRegistryError::Db(error.to_string()))?
            .ok_or(ClientRegistryError::Unknown)?;

        binding_from_statuses(
            &client.client_id,
            &client.tenant_slug,
            &client.portal,
            &client.status,
            &tenant.status,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_client_and_tenant_produce_binding() {
        let binding = binding_from_statuses(
            "hauliage-web",
            "hauliage",
            "frontend",
            "active",
            "active",
        )
        .expect("active binding");

        assert_eq!(binding.client_id, "hauliage-web");
        assert_eq!(binding.tenant_id, "hauliage");
        assert_eq!(binding.portal, "frontend");
    }

    #[test]
    fn disabled_client_is_rejected_without_tenant_disclosure() {
        assert_eq!(
            binding_from_statuses(
                "hauliage-web",
                "hauliage",
                "frontend",
                "disabled",
                "active",
            ),
            Err(ClientRegistryError::Unknown)
        );
    }

    #[test]
    fn suspended_tenant_is_rejected() {
        assert_eq!(
            binding_from_statuses(
                "hauliage-web",
                "hauliage",
                "frontend",
                "active",
                "suspended",
            ),
            Err(ClientRegistryError::NotActive)
        );
    }
}
