//! Tenant-owned OIDC relying-party lifecycle.

use chrono::{DateTime, Duration, Utc};
use lifeguard::active_model::ActiveModelTrait;
use lifeguard::{ColumnTrait, LifeExecutor, LifeModelTrait};
use sea_query::Values;
use sesame_common::oidc_client::{
    hash_client_secret, normalize_redirect_uri, ClientPolicy, ClientSecret, ClientType,
    TokenEndpointAuthMethod,
};
use uuid::Uuid;

use crate::models::relying_party_client::{
    Column as ClientColumn, Entity as ClientEntity, RelyingPartyClientModel,
    RelyingPartyClientRecord,
};
use crate::models::relying_party_client_capability::{
    Column as CapabilityColumn, Entity as CapabilityEntity, RelyingPartyClientCapabilityRecord,
};
use crate::models::relying_party_client_redirect_uri::{
    Column as RedirectColumn, Entity as RedirectEntity, RelyingPartyClientRedirectUriRecord,
};
use crate::models::relying_party_client_secret::{
    Column as SecretColumn, Entity as SecretEntity, RelyingPartyClientSecretRecord,
};

const AUTHORITY_TENANT: &str = "tenant";

#[derive(Debug, Clone)]
pub struct CreateClientInput {
    pub application_id: String,
    pub client_type: String,
    pub token_endpoint_auth_method: Option<String>,
    pub redirect_uris: Vec<String>,
    pub post_logout_redirect_uris: Vec<String>,
    pub grants: Vec<String>,
    pub response_types: Vec<String>,
    pub scopes: Vec<String>,
    pub audiences: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateClientInput {
    pub status: Option<String>,
    pub redirect_uris: Option<Vec<String>>,
    pub post_logout_redirect_uris: Option<Vec<String>>,
    pub grants: Option<Vec<String>>,
    pub response_types: Option<Vec<String>>,
    pub scopes: Option<Vec<String>>,
    pub audiences: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientView {
    pub client_id: String,
    pub tenant_id: String,
    pub application_id: String,
    pub client_type: String,
    pub token_endpoint_auth_method: String,
    pub pkce_s256_required: bool,
    pub status: String,
    pub redirect_uris: Vec<String>,
    pub post_logout_redirect_uris: Vec<String>,
    pub grants: Vec<String>,
    pub response_types: Vec<String>,
    pub scopes: Vec<String>,
    pub audiences: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct CreatedClient {
    pub client: ClientView,
    pub client_secret: Option<ClientSecret>,
    pub secret_id: Option<Uuid>,
}

#[derive(Debug)]
pub struct RotatedSecret {
    pub client_id: String,
    pub secret_id: Uuid,
    pub client_secret: ClientSecret,
    pub created_at: DateTime<Utc>,
    pub previous_secrets_valid_until: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientAdminError {
    InvalidPolicy(String),
    NotFound,
    PublicClientHasNoSecret,
    InvalidOverlap,
    Db(String),
}

impl ClientAdminError {
    #[must_use]
    pub fn status(&self) -> u16 {
        match self {
            Self::InvalidPolicy(_) | Self::PublicClientHasNoSecret | Self::InvalidOverlap => 400,
            Self::NotFound => 404,
            Self::Db(_) => 500,
        }
    }
}

#[derive(Debug)]
struct PreparedClient {
    application_id: String,
    client_type: ClientType,
    token_endpoint_auth_method: TokenEndpointAuthMethod,
    pkce_s256_required: bool,
    redirect_uris: Vec<String>,
    post_logout_redirect_uris: Vec<String>,
    policy: ClientPolicy,
}

fn prepare_client(input: CreateClientInput) -> Result<PreparedClient, ClientAdminError> {
    let application_id = input.application_id.trim();
    if application_id.is_empty() || application_id.len() > 64 {
        return Err(ClientAdminError::InvalidPolicy(
            "application_id must contain 1 to 64 characters".to_string(),
        ));
    }

    let client_type = match input.client_type.as_str() {
        "public" => ClientType::Public,
        "confidential" => ClientType::Confidential,
        _ => {
            return Err(ClientAdminError::InvalidPolicy(
                "unsupported client_type".to_string(),
            ))
        }
    };
    let default_method = match client_type {
        ClientType::Public => "none",
        ClientType::Confidential => "client_secret_basic",
    };
    let method = input
        .token_endpoint_auth_method
        .as_deref()
        .unwrap_or(default_method);
    let token_endpoint_auth_method = match method {
        "none" => TokenEndpointAuthMethod::None,
        "client_secret_basic" => TokenEndpointAuthMethod::ClientSecretBasic,
        "client_secret_post" => TokenEndpointAuthMethod::ClientSecretPost,
        _ => {
            return Err(ClientAdminError::InvalidPolicy(
                "unsupported token endpoint authentication method".to_string(),
            ))
        }
    };
    let pkce_s256_required = client_type == ClientType::Public;
    let policy = ClientPolicy {
        client_type,
        token_endpoint_auth_method,
        pkce_s256_required,
        grants: deduplicate(input.grants),
        response_types: deduplicate(input.response_types),
        scopes: deduplicate(input.scopes),
        audiences: deduplicate(input.audiences),
    };
    policy
        .validate()
        .map_err(|error| ClientAdminError::InvalidPolicy(error.to_string()))?;

    Ok(PreparedClient {
        application_id: application_id.to_string(),
        client_type,
        token_endpoint_auth_method,
        pkce_s256_required,
        redirect_uris: normalize_redirects(input.redirect_uris, true)?,
        post_logout_redirect_uris: normalize_redirects(input.post_logout_redirect_uris, false)?,
        policy,
    })
}

fn normalize_redirects(
    redirects: Vec<String>,
    required: bool,
) -> Result<Vec<String>, ClientAdminError> {
    let mut normalized = redirects
        .iter()
        .map(|redirect| {
            normalize_redirect_uri(redirect)
                .map_err(|error| ClientAdminError::InvalidPolicy(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    normalized.sort();
    normalized.dedup();
    if required && normalized.is_empty() {
        return Err(ClientAdminError::InvalidPolicy(
            "at least one login redirect URI is required".to_string(),
        ));
    }
    Ok(normalized)
}

fn deduplicate(mut values: Vec<String>) -> Vec<String> {
    values.iter_mut().for_each(|value| {
        *value = value.trim().to_string();
    });
    values.sort();
    values.dedup();
    values
}

fn client_type_name(client_type: ClientType) -> &'static str {
    match client_type {
        ClientType::Public => "public",
        ClientType::Confidential => "confidential",
    }
}

fn auth_method_name(method: TokenEndpointAuthMethod) -> &'static str {
    match method {
        TokenEndpointAuthMethod::None => "none",
        TokenEndpointAuthMethod::ClientSecretBasic => "client_secret_basic",
        TokenEndpointAuthMethod::ClientSecretPost => "client_secret_post",
    }
}

fn delete_client_row<E: LifeExecutor>(client_id: Uuid, exec: &E) {
    let _ = LifeExecutor::execute_values(
        exec,
        "DELETE FROM sesame_idam.relying_party_clients WHERE id = $1",
        &Values(vec![client_id.into()]),
    );
}

fn insert_redirects<E: LifeExecutor>(
    client_id: Uuid,
    kind: &str,
    redirects: &[String],
    now: DateTime<Utc>,
    exec: &E,
) -> Result<(), ClientAdminError> {
    for redirect in redirects {
        let mut record = RelyingPartyClientRedirectUriRecord::new();
        record
            .set_id(Uuid::new_v4())
            .set_relying_party_client_id(client_id)
            .set_kind(kind.to_string())
            .set_uri(redirect.clone())
            .set_created_at(now);
        record
            .insert(exec)
            .map_err(|error| ClientAdminError::Db(error.to_string()))?;
    }
    Ok(())
}

fn insert_capabilities<E: LifeExecutor>(
    client_id: Uuid,
    policy: &ClientPolicy,
    now: DateTime<Utc>,
    exec: &E,
) -> Result<(), ClientAdminError> {
    for (kind, values) in [
        ("grant", &policy.grants),
        ("response_type", &policy.response_types),
        ("scope", &policy.scopes),
        ("audience", &policy.audiences),
    ] {
        for value in values {
            let mut record = RelyingPartyClientCapabilityRecord::new();
            record
                .set_id(Uuid::new_v4())
                .set_relying_party_client_id(client_id)
                .set_kind(kind.to_string())
                .set_value(value.clone())
                .set_created_at(now);
            record
                .insert(exec)
                .map_err(|error| ClientAdminError::Db(error.to_string()))?;
        }
    }
    Ok(())
}

fn find_tenant_client<E: LifeExecutor>(
    tenant_id: &str,
    client_id: &str,
    exec: &E,
) -> Result<RelyingPartyClientModel, ClientAdminError> {
    ClientEntity::find()
        .filter(ClientColumn::TenantSlug.eq(tenant_id.to_string()))
        .filter(ClientColumn::ClientId.eq(client_id.to_string()))
        .filter(ClientColumn::AuthorityClass.eq(AUTHORITY_TENANT.to_string()))
        .find_one(exec)
        .map_err(|error| ClientAdminError::Db(error.to_string()))?
        .ok_or(ClientAdminError::NotFound)
}

fn build_view<E: LifeExecutor>(
    client: &RelyingPartyClientModel,
    exec: &E,
) -> Result<ClientView, ClientAdminError> {
    let redirects = RedirectEntity::find()
        .filter(RedirectColumn::RelyingPartyClientId.eq(client.id))
        .all(exec)
        .map_err(|error| ClientAdminError::Db(error.to_string()))?;
    let capabilities = CapabilityEntity::find()
        .filter(CapabilityColumn::RelyingPartyClientId.eq(client.id))
        .all(exec)
        .map_err(|error| ClientAdminError::Db(error.to_string()))?;
    let values = |kind: &str| {
        let mut values = capabilities
            .iter()
            .filter(|capability| capability.kind == kind)
            .map(|capability| capability.value.clone())
            .collect::<Vec<_>>();
        values.sort();
        values
    };
    let redirect_values = |kind: &str| {
        let mut values = redirects
            .iter()
            .filter(|redirect| redirect.kind == kind)
            .map(|redirect| redirect.uri.clone())
            .collect::<Vec<_>>();
        values.sort();
        values
    };

    Ok(ClientView {
        client_id: client.client_id.clone(),
        tenant_id: client.tenant_slug.clone(),
        application_id: client.application_id.clone(),
        client_type: client.client_type.clone(),
        token_endpoint_auth_method: client.token_endpoint_auth_method.clone(),
        pkce_s256_required: client.pkce_s256_required,
        status: client.status.clone(),
        redirect_uris: redirect_values("login"),
        post_logout_redirect_uris: redirect_values("post_logout"),
        grants: values("grant"),
        response_types: values("response_type"),
        scopes: values("scope"),
        audiences: values("audience"),
        created_at: client.created_at,
        updated_at: client.updated_at,
    })
}

pub struct OidcClientAdmin;

impl OidcClientAdmin {
    pub fn create<E: LifeExecutor>(
        tenant_id: &str,
        input: CreateClientInput,
        exec: &E,
    ) -> Result<CreatedClient, ClientAdminError> {
        let prepared = prepare_client(input)?;
        let now = Utc::now();
        let database_id = Uuid::new_v4();
        let client_id = format!("ses_{}", Uuid::new_v4().simple());
        let mut record = RelyingPartyClientRecord::new();
        record
            .set_id(database_id)
            .set_client_id(client_id.clone())
            .set_tenant_slug(tenant_id.to_string())
            .set_portal(prepared.application_id.clone())
            .set_application_id(prepared.application_id)
            .set_client_type(client_type_name(prepared.client_type).to_string())
            .set_token_endpoint_auth_method(
                auth_method_name(prepared.token_endpoint_auth_method).to_string(),
            )
            .set_pkce_s256_required(prepared.pkce_s256_required)
            .set_authority_class(AUTHORITY_TENANT.to_string())
            .set_status("active".to_string())
            .set_created_at(now)
            .set_updated_at(now);
        record
            .insert(exec)
            .map_err(|error| ClientAdminError::Db(error.to_string()))?;

        let insert_result =
            insert_redirects(database_id, "login", &prepared.redirect_uris, now, exec)
                .and_then(|()| {
                    insert_redirects(
                        database_id,
                        "post_logout",
                        &prepared.post_logout_redirect_uris,
                        now,
                        exec,
                    )
                })
                .and_then(|()| insert_capabilities(database_id, &prepared.policy, now, exec));
        if let Err(error) = insert_result {
            delete_client_row(database_id, exec);
            return Err(error);
        }

        let (client_secret, secret_id) = if prepared.client_type == ClientType::Confidential {
            let secret = ClientSecret::generate();
            let hash = hash_client_secret(secret.expose_once())
                .map_err(ClientAdminError::InvalidPolicy)?;
            let secret_id = Uuid::new_v4();
            let mut secret_record = RelyingPartyClientSecretRecord::new();
            secret_record
                .set_id(secret_id)
                .set_relying_party_client_id(database_id)
                .set_secret_hash(hash)
                .set_status("active".to_string())
                .set_valid_from(now)
                .set_valid_until(None)
                .set_created_at(now)
                .set_revoked_at(None);
            if let Err(error) = secret_record.insert(exec) {
                delete_client_row(database_id, exec);
                return Err(ClientAdminError::Db(error.to_string()));
            }
            (Some(secret), Some(secret_id))
        } else {
            (None, None)
        };

        let client = find_tenant_client(tenant_id, &client_id, exec)?;
        Ok(CreatedClient {
            client: build_view(&client, exec)?,
            client_secret,
            secret_id,
        })
    }

    pub fn list<E: LifeExecutor>(
        tenant_id: &str,
        exec: &E,
    ) -> Result<Vec<ClientView>, ClientAdminError> {
        let mut clients = ClientEntity::find()
            .filter(ClientColumn::TenantSlug.eq(tenant_id.to_string()))
            .filter(ClientColumn::AuthorityClass.eq(AUTHORITY_TENANT.to_string()))
            .all(exec)
            .map_err(|error| ClientAdminError::Db(error.to_string()))?;
        clients.retain(|client| client.status != "deleted");
        clients.sort_by_key(|client| client.created_at);
        clients
            .iter()
            .map(|client| build_view(client, exec))
            .collect()
    }

    pub fn get<E: LifeExecutor>(
        tenant_id: &str,
        client_id: &str,
        exec: &E,
    ) -> Result<ClientView, ClientAdminError> {
        let client = find_tenant_client(tenant_id, client_id, exec)?;
        if client.status == "deleted" {
            return Err(ClientAdminError::NotFound);
        }
        build_view(&client, exec)
    }

    pub fn update<E: LifeExecutor>(
        tenant_id: &str,
        client_id: &str,
        input: UpdateClientInput,
        exec: &E,
    ) -> Result<ClientView, ClientAdminError> {
        let client = find_tenant_client(tenant_id, client_id, exec)?;
        if client.status == "deleted" {
            return Err(ClientAdminError::NotFound);
        }
        let current = build_view(&client, exec)?;
        let prepared = prepare_client(CreateClientInput {
            application_id: current.application_id,
            client_type: current.client_type,
            token_endpoint_auth_method: Some(current.token_endpoint_auth_method),
            redirect_uris: input.redirect_uris.unwrap_or(current.redirect_uris),
            post_logout_redirect_uris: input
                .post_logout_redirect_uris
                .unwrap_or(current.post_logout_redirect_uris),
            grants: input.grants.unwrap_or(current.grants),
            response_types: input.response_types.unwrap_or(current.response_types),
            scopes: input.scopes.unwrap_or(current.scopes),
            audiences: input.audiences.unwrap_or(current.audiences),
        })?;
        let status = input.status.unwrap_or(client.status.clone());
        if !matches!(status.as_str(), "active" | "disabled") {
            return Err(ClientAdminError::InvalidPolicy(
                "status must be active or disabled".to_string(),
            ));
        }

        let now = Utc::now();
        let mut record = RelyingPartyClientRecord::new();
        record
            .set_id(client.id)
            .set_client_id(client.client_id.clone())
            .set_tenant_slug(client.tenant_slug.clone())
            .set_portal(client.portal.clone())
            .set_application_id(client.application_id.clone())
            .set_client_type(client.client_type.clone())
            .set_token_endpoint_auth_method(client.token_endpoint_auth_method.clone())
            .set_pkce_s256_required(client.pkce_s256_required)
            .set_authority_class(client.authority_class.clone())
            .set_status(status)
            .set_created_at(client.created_at)
            .set_updated_at(now);
        record
            .update(exec)
            .map_err(|error| ClientAdminError::Db(error.to_string()))?;

        LifeExecutor::execute_values(
            exec,
            "DELETE FROM sesame_idam.relying_party_client_redirect_uris WHERE relying_party_client_id = $1",
            &Values(vec![client.id.into()]),
        )
        .map_err(|error| ClientAdminError::Db(error.to_string()))?;
        LifeExecutor::execute_values(
            exec,
            "DELETE FROM sesame_idam.relying_party_client_capabilities WHERE relying_party_client_id = $1",
            &Values(vec![client.id.into()]),
        )
        .map_err(|error| ClientAdminError::Db(error.to_string()))?;
        insert_redirects(client.id, "login", &prepared.redirect_uris, now, exec)?;
        insert_redirects(
            client.id,
            "post_logout",
            &prepared.post_logout_redirect_uris,
            now,
            exec,
        )?;
        insert_capabilities(client.id, &prepared.policy, now, exec)?;
        Self::get(tenant_id, client_id, exec)
    }

    pub fn delete<E: LifeExecutor>(
        tenant_id: &str,
        client_id: &str,
        exec: &E,
    ) -> Result<(), ClientAdminError> {
        let client = find_tenant_client(tenant_id, client_id, exec)?;
        if client.status == "deleted" {
            return Err(ClientAdminError::NotFound);
        }
        let now = Utc::now();
        let mut record = RelyingPartyClientRecord::new();
        record
            .set_id(client.id)
            .set_client_id(client.client_id)
            .set_tenant_slug(client.tenant_slug)
            .set_portal(client.portal)
            .set_application_id(client.application_id)
            .set_client_type(client.client_type)
            .set_token_endpoint_auth_method(client.token_endpoint_auth_method)
            .set_pkce_s256_required(client.pkce_s256_required)
            .set_authority_class(client.authority_class)
            .set_status("deleted".to_string())
            .set_created_at(client.created_at)
            .set_updated_at(now);
        record
            .update(exec)
            .map_err(|error| ClientAdminError::Db(error.to_string()))?;
        LifeExecutor::execute_values(
            exec,
            "UPDATE sesame_idam.relying_party_client_secrets SET status = 'revoked', revoked_at = $2, valid_until = $2 WHERE relying_party_client_id = $1 AND status = 'active'",
            &Values(vec![client.id.into(), now.into()]),
        )
        .map_err(|error| ClientAdminError::Db(error.to_string()))?;
        Ok(())
    }

    pub fn rotate_secret<E: LifeExecutor>(
        tenant_id: &str,
        client_id: &str,
        overlap_seconds: i64,
        exec: &E,
    ) -> Result<RotatedSecret, ClientAdminError> {
        if !(0..=86_400).contains(&overlap_seconds) {
            return Err(ClientAdminError::InvalidOverlap);
        }
        let client = find_tenant_client(tenant_id, client_id, exec)?;
        if client.status == "deleted" {
            return Err(ClientAdminError::NotFound);
        }
        if client.client_type != "confidential" {
            return Err(ClientAdminError::PublicClientHasNoSecret);
        }

        let now = Utc::now();
        let previous_valid_until = now + Duration::seconds(overlap_seconds);
        LifeExecutor::execute_values(
            exec,
            "UPDATE sesame_idam.relying_party_client_secrets SET valid_until = $2 WHERE relying_party_client_id = $1 AND status = 'active' AND (valid_until IS NULL OR valid_until > $2)",
            &Values(vec![client.id.into(), previous_valid_until.into()]),
        )
        .map_err(|error| ClientAdminError::Db(error.to_string()))?;

        let secret = ClientSecret::generate();
        let secret_hash =
            hash_client_secret(secret.expose_once()).map_err(ClientAdminError::InvalidPolicy)?;
        let secret_id = Uuid::new_v4();
        let mut record = RelyingPartyClientSecretRecord::new();
        record
            .set_id(secret_id)
            .set_relying_party_client_id(client.id)
            .set_secret_hash(secret_hash)
            .set_status("active".to_string())
            .set_valid_from(now)
            .set_valid_until(None)
            .set_created_at(now)
            .set_revoked_at(None);
        record
            .insert(exec)
            .map_err(|error| ClientAdminError::Db(error.to_string()))?;

        Ok(RotatedSecret {
            client_id: client.client_id,
            secret_id,
            client_secret: secret,
            created_at: now,
            previous_secrets_valid_until: previous_valid_until,
        })
    }

    pub fn revoke_secret<E: LifeExecutor>(
        tenant_id: &str,
        client_id: &str,
        secret_id: Uuid,
        exec: &E,
    ) -> Result<(), ClientAdminError> {
        let client = find_tenant_client(tenant_id, client_id, exec)?;
        let secret = SecretEntity::find()
            .filter(SecretColumn::Id.eq(secret_id))
            .filter(SecretColumn::RelyingPartyClientId.eq(client.id))
            .find_one(exec)
            .map_err(|error| ClientAdminError::Db(error.to_string()))?
            .ok_or(ClientAdminError::NotFound)?;
        let now = Utc::now();
        let mut record = RelyingPartyClientSecretRecord::new();
        record
            .set_id(secret.id)
            .set_relying_party_client_id(secret.relying_party_client_id)
            .set_secret_hash(secret.secret_hash)
            .set_status("revoked".to_string())
            .set_valid_from(secret.valid_from)
            .set_valid_until(Some(now))
            .set_created_at(secret.created_at)
            .set_revoked_at(Some(now));
        record
            .update(exec)
            .map_err(|error| ClientAdminError::Db(error.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn public_input() -> CreateClientInput {
        CreateClientInput {
            application_id: "portal".to_string(),
            client_type: "public".to_string(),
            token_endpoint_auth_method: None,
            redirect_uris: vec!["https://EXAMPLE.com:443/callback".to_string()],
            post_logout_redirect_uris: vec!["https://example.com/signed-out".to_string()],
            grants: vec![
                "refresh_token".to_string(),
                "authorization_code".to_string(),
            ],
            response_types: vec!["code".to_string()],
            scopes: vec!["profile".to_string(), "openid".to_string()],
            audiences: vec!["sesame-idam".to_string()],
        }
    }

    #[test]
    fn public_client_policy_is_pkce_only_and_normalized() {
        let prepared = prepare_client(public_input()).expect("valid client");
        assert_eq!(prepared.client_type, ClientType::Public);
        assert_eq!(
            prepared.token_endpoint_auth_method,
            TokenEndpointAuthMethod::None
        );
        assert!(prepared.pkce_s256_required);
        assert_eq!(prepared.redirect_uris, ["https://example.com/callback"]);
    }

    #[test]
    fn public_client_cannot_select_secret_authentication() {
        let mut input = public_input();
        input.token_endpoint_auth_method = Some("client_secret_post".to_string());
        assert!(matches!(
            prepare_client(input),
            Err(ClientAdminError::InvalidPolicy(_))
        ));
    }

    #[test]
    fn unsafe_redirect_is_rejected_before_database_access() {
        let mut input = public_input();
        input.redirect_uris = vec!["https://*.example.com/callback".to_string()];
        assert!(matches!(
            prepare_client(input),
            Err(ClientAdminError::InvalidPolicy(_))
        ));
    }
}
