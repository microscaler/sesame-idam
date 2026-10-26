use lifeguard::{ColumnTrait, LifeExecutor, LifeModelTrait};
use sesame_common::oidc_client::{
    hash_client_secret, verify_client_secret, ClientPolicy, ClientPolicyError, ClientType,
    TokenEndpointAuthMethod,
};

use crate::models::relying_party_client::{Column as ClientColumn, Entity as ClientEntity};
use crate::models::relying_party_client_capability::{
    Column as CapabilityColumn, Entity as CapabilityEntity,
};
use crate::models::relying_party_client_redirect_uri::{
    Column as RedirectColumn, Entity as RedirectEntity,
};
use crate::models::relying_party_client_secret::{Column as SecretColumn, Entity as SecretEntity};
use crate::services::tenant_service::TenantService;

pub const STATUS_ACTIVE: &str = "active";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientBinding {
    pub client_id: String,
    pub tenant_id: String,
    pub portal: String,
    pub application_id: String,
    pub authority_class: String,
    pub policy: ClientPolicy,
    pub redirect_uris: Vec<String>,
    pub post_logout_redirect_uris: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientRegistryError {
    Unknown,
    NotActive,
    InvalidPolicy(String),
    Db(String),
}

fn policy_from_parts(
    client_type: &str,
    token_endpoint_auth_method: &str,
    pkce_s256_required: bool,
    capabilities: &[(&str, &str)],
) -> Result<ClientPolicy, ClientPolicyError> {
    let client_type = match client_type {
        "public" => ClientType::Public,
        "confidential" => ClientType::Confidential,
        _ => return Err(ClientPolicyError::UnsupportedCapability),
    };
    let token_endpoint_auth_method = match token_endpoint_auth_method {
        "none" => TokenEndpointAuthMethod::None,
        "client_secret_basic" => TokenEndpointAuthMethod::ClientSecretBasic,
        "client_secret_post" => TokenEndpointAuthMethod::ClientSecretPost,
        _ => return Err(ClientPolicyError::UnsupportedCapability),
    };

    let values = |kind: &str| {
        capabilities
            .iter()
            .filter(|(candidate, _)| *candidate == kind)
            .map(|(_, value)| (*value).to_string())
            .collect::<Vec<_>>()
    };
    let policy = ClientPolicy {
        client_type,
        token_endpoint_auth_method,
        pkce_s256_required,
        grants: values("grant"),
        response_types: values("response_type"),
        scopes: values("scope"),
        audiences: values("audience"),
    };
    policy.validate()?;
    Ok(policy)
}

fn secret_is_usable(
    status: &str,
    valid_from: chrono::DateTime<chrono::Utc>,
    valid_until: Option<chrono::DateTime<chrono::Utc>>,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    status == STATUS_ACTIVE && valid_from <= now && valid_until.is_none_or(|until| until > now)
}

fn conceal_secret_verification_timing(secret: &str) {
    static DUMMY_SECRET_HASH: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
        hash_client_secret("ses_dummy_client_secret")
            .expect("static OIDC client timing hash must be constructible")
    });
    let _ = verify_client_secret(secret, &DUMMY_SECRET_HASH);
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
        application_id: portal.to_string(),
        authority_class: "tenant".to_string(),
        policy: ClientPolicy {
            client_type: ClientType::Public,
            token_endpoint_auth_method: TokenEndpointAuthMethod::None,
            pkce_s256_required: true,
            grants: vec!["authorization_code".to_string()],
            response_types: vec!["code".to_string()],
            scopes: vec!["openid".to_string()],
            audiences: vec!["sesame-idam".to_string()],
        },
        redirect_uris: Vec::new(),
        post_logout_redirect_uris: Vec::new(),
    })
}

pub struct ClientRegistry;

impl ClientRegistry {
    /// Resolve an active registered client into trusted tenant/application context.
    pub fn resolve_active<E: LifeExecutor>(
        client_id: &str,
        exec: &E,
    ) -> Result<ClientBinding, ClientRegistryError> {
        Self::resolve_active_record(client_id, exec).map(|(binding, _)| binding)
    }

    /// [`Self::resolve_active`], with the client's row id (for its secrets).
    pub fn resolve_active_record<E: LifeExecutor>(
        client_id: &str,
        exec: &E,
    ) -> Result<(ClientBinding, uuid::Uuid), ClientRegistryError> {
        let client_id = client_id.trim();
        if client_id.is_empty() {
            return Err(ClientRegistryError::Unknown);
        }

        let client = ClientEntity::find()
            .filter(ClientColumn::ClientId.eq(client_id.to_string()))
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
        )?;

        let capabilities = CapabilityEntity::find()
            .filter(CapabilityColumn::RelyingPartyClientId.eq(client.id))
            .all(exec)
            .map_err(|error| ClientRegistryError::Db(error.to_string()))?;
        let capability_parts = capabilities
            .iter()
            .map(|capability| (capability.kind.as_str(), capability.value.as_str()))
            .collect::<Vec<_>>();
        let policy = policy_from_parts(
            &client.client_type,
            &client.token_endpoint_auth_method,
            client.pkce_s256_required,
            &capability_parts,
        )
        .map_err(|error| ClientRegistryError::InvalidPolicy(error.to_string()))?;

        let redirects = RedirectEntity::find()
            .filter(RedirectColumn::RelyingPartyClientId.eq(client.id))
            .all(exec)
            .map_err(|error| ClientRegistryError::Db(error.to_string()))?;

        let record_id = client.id;
        Ok((
            ClientBinding {
                client_id: client.client_id,
                tenant_id: client.tenant_slug,
                portal: client.portal,
                application_id: client.application_id,
                authority_class: client.authority_class,
                policy,
                redirect_uris: redirects
                    .iter()
                    .filter(|redirect| redirect.kind == "login")
                    .map(|redirect| redirect.uri.clone())
                    .collect(),
                post_logout_redirect_uris: redirects
                    .iter()
                    .filter(|redirect| redirect.kind == "post_logout")
                    .map(|redirect| redirect.uri.clone())
                    .collect(),
            },
            record_id,
        ))
    }

    /// Resolve tenant/application for public pre-auth north–south handlers.
    ///
    /// Active `client_id` is authoritative. When optional legacy `X-Tenant-ID`
    /// is present (non-empty after trim), it must match the registered client
    /// tenant; mismatches map to [`ClientRegistryError::Unknown`] so callers
    /// can return the same `invalid_client` / `client_invalid` shape as an
    /// unknown client (no tenant oracle).
    pub fn resolve_pre_auth<E: LifeExecutor>(
        client_id: &str,
        x_tenant_id: Option<&str>,
        exec: &E,
    ) -> Result<ClientBinding, ClientRegistryError> {
        let binding = Self::resolve_active(client_id, exec)?;
        if x_tenant_id
            .map(str::trim)
            .filter(|tenant| !tenant.is_empty())
            .is_some_and(|tenant| tenant != binding.tenant_id)
        {
            return Err(ClientRegistryError::Unknown);
        }
        Ok(binding)
    }

    /// Authenticate a confidential client without disclosing whether its id,
    /// status, policy, or secret caused rejection: every rejection costs one argon2id verification.
    pub fn authenticate_confidential<E: LifeExecutor>(
        client_id: &str,
        presented_secret: &str,
        exec: &E,
    ) -> Result<ClientBinding, ClientRegistryError> {
        let (binding, record_id) = match Self::resolve_active_record(client_id, exec) {
            Ok(resolved) => resolved,
            Err(error) => {
                conceal_secret_verification_timing(presented_secret);
                return Err(error);
            }
        };
        if binding.policy.client_type != ClientType::Confidential {
            conceal_secret_verification_timing(presented_secret);
            return Err(ClientRegistryError::Unknown);
        }
        if Self::verify_secret(record_id, presented_secret, exec) {
            Ok(binding)
        } else {
            Err(ClientRegistryError::Unknown)
        }
    }

    /// Whether `presented_secret` matches one of the client's usable secrets. Every call costs at least one argon2id
    /// verification: a real one per usable secret tried, or one against the dummy hash when there is none (or the
    /// lookup failed), so a client without a usable secret answers as slowly as one with a wrong secret.
    pub fn verify_secret<E: LifeExecutor>(
        record_id: uuid::Uuid,
        presented_secret: &str,
        exec: &E,
    ) -> bool {
        let now = chrono::Utc::now();
        let secrets = SecretEntity::find()
            .filter(SecretColumn::RelyingPartyClientId.eq(record_id))
            .all(exec)
            .unwrap_or_default();
        let mut verified = 0_usize;
        let matched = secrets
            .iter()
            .filter(|secret| {
                secret_is_usable(&secret.status, secret.valid_from, secret.valid_until, now)
            })
            .any(|secret| {
                verified += 1;
                verify_client_secret(presented_secret, &secret.secret_hash)
            });
        if verified == 0 {
            conceal_secret_verification_timing(presented_secret);
        }
        matched
    }

    /// One argon2id verification against a fixed hash: the cost of checking a secret, spent on a path that
    /// rejects without one, so "no such client" and "wrong secret" take the same time.
    pub fn conceal_timing(presented_secret: &str) {
        conceal_secret_verification_timing(presented_secret);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_client_and_tenant_produce_binding() {
        let binding = binding_from_statuses("acme-web", "acme", "frontend", "active", "active")
            .expect("active binding");

        assert_eq!(binding.client_id, "acme-web");
        assert_eq!(binding.tenant_id, "acme");
        assert_eq!(binding.portal, "frontend");
    }

    #[test]
    fn disabled_client_is_rejected_without_tenant_disclosure() {
        assert_eq!(
            binding_from_statuses("acme-web", "acme", "frontend", "disabled", "active",),
            Err(ClientRegistryError::Unknown)
        );
    }

    #[test]
    fn suspended_tenant_is_rejected() {
        assert_eq!(
            binding_from_statuses("acme-web", "acme", "frontend", "active", "suspended",),
            Err(ClientRegistryError::NotActive)
        );
    }

    #[test]
    fn complete_public_policy_is_derived_from_registered_capabilities() {
        let policy = policy_from_parts(
            "public",
            "none",
            true,
            &[
                ("grant", "authorization_code"),
                ("grant", "refresh_token"),
                ("response_type", "code"),
                ("scope", "openid"),
                ("scope", "profile"),
                ("audience", "sesame-idam"),
            ],
        )
        .expect("valid public policy");

        assert_eq!(
            policy.client_type,
            sesame_common::oidc_client::ClientType::Public
        );
        assert!(policy.pkce_s256_required);
        assert_eq!(policy.grants, ["authorization_code", "refresh_token"]);
    }

    #[test]
    fn active_secret_respects_rotation_overlap_and_revocation() {
        let now = chrono::Utc::now();
        assert!(secret_is_usable(
            "active",
            now - chrono::Duration::minutes(1),
            Some(now + chrono::Duration::minutes(5)),
            now,
        ));
        assert!(!secret_is_usable(
            "revoked",
            now - chrono::Duration::minutes(1),
            Some(now + chrono::Duration::minutes(5)),
            now,
        ));
        assert!(!secret_is_usable(
            "active",
            now - chrono::Duration::minutes(10),
            Some(now - chrono::Duration::seconds(1)),
            now,
        ));
    }
}
