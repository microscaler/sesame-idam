//! Standards-compliant OIDC token endpoint.

use base64::Engine;
use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_common::oidc_client::{ClientType, TokenEndpointAuthMethod};
use sesame_idam_identity_login_service_gen::handlers::oauth_token::Request;
use sesame_idam_identity_session_service::services::token_rotation::{
    rotate_refresh_token_for_client, RotationOutcome,
};

use crate::services::client_registry::{ClientBinding, ClientRegistry};
use crate::services::oidc_authorization::{redeem_code, LookupError};
use crate::services::token_issuer::{issue_id_token, issue_tokens_for_client};

#[handler(OauthTokenController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let exec = sesame_idam_database::db();
    let binding = match authenticate_client(&req.data, exec) {
        Ok(binding) => binding,
        Err(()) => return oauth_error(401, "invalid_client", "Client authentication failed"),
    };
    if !binding
        .policy
        .grants
        .iter()
        .any(|grant| grant == &req.data.grant_type)
    {
        return oauth_error(
            400,
            "unauthorized_client",
            "The client is not permitted to use this grant",
        );
    }

    match req.data.grant_type.as_str() {
        "authorization_code" => exchange_code(&req.data, &binding),
        "refresh_token" => exchange_refresh(&req.data, &binding),
        _ => oauth_error(400, "unsupported_grant_type", "Unsupported grant_type"),
    }
}

fn authenticate_client<E: lifeguard::LifeExecutor>(
    request: &Request,
    exec: &E,
) -> Result<ClientBinding, ()> {
    let basic = request.authorization.as_deref().and_then(parse_basic);
    authenticate(
        basic
            .as_ref()
            .map(|(id, secret)| (id.as_str(), secret.as_str())),
        request.client_id.as_deref(),
        request.client_secret.as_deref(),
        |client_id| ClientRegistry::resolve_active_record(client_id, exec).ok(),
        |record_id, secret| ClientRegistry::verify_secret(record_id, secret, exec),
        ClientRegistry::conceal_timing,
    )
}

/// Client authentication at the token endpoint, with the lookups passed in (unit-tested).
///
/// The client is resolved once (it used to be resolved again inside the secret check: ten queries, now five).
/// Every rejection of a presented secret costs one argon2id verification, whatever the cause - an unknown or
/// inactive client, a client of the wrong type or method, or a wrong secret - so the response time does not say
/// which. (Before 26 Oct 2026 an unknown client answered without any verification: a client-id oracle.)
fn authenticate<Id: Copy>(
    basic: Option<(&str, &str)>,
    client_id_param: Option<&str>,
    client_secret_param: Option<&str>,
    resolve: impl FnOnce(&str) -> Option<(ClientBinding, Id)>,
    verify: impl FnOnce(Id, &str) -> bool,
    conceal: impl Fn(&str),
) -> Result<ClientBinding, ()> {
    let presented = basic.map(|(_, s)| s).or(client_secret_param);
    let reject = || {
        if let Some(secret) = presented {
            conceal(secret);
        }
        Err(())
    };
    let Some(client_id) = basic.map(|(id, _)| id).or(client_id_param) else {
        return reject();
    };
    let Some((binding, record_id)) = resolve(client_id) else {
        return reject();
    };
    let secret = match (
        &binding.policy.client_type,
        &binding.policy.token_endpoint_auth_method,
    ) {
        (ClientType::Public, TokenEndpointAuthMethod::None) if presented.is_none() => {
            return Ok(binding);
        }
        (ClientType::Confidential, TokenEndpointAuthMethod::ClientSecretBasic) => {
            basic.map(|(_, s)| s)
        }
        (ClientType::Confidential, TokenEndpointAuthMethod::ClientSecretPost)
            if basic.is_none() =>
        {
            client_secret_param
        }
        _ => None,
    };
    match secret {
        Some(secret) => {
            if verify(record_id, secret) {
                Ok(binding)
            } else {
                Err(())
            }
        }
        None => reject(),
    }
}

fn parse_basic(header: &str) -> Option<(String, String)> {
    let encoded = header.strip_prefix("Basic ")?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    let decoded = String::from_utf8(decoded).ok()?;
    let (client_id, secret) = decoded.split_once(':')?;
    Some((client_id.to_string(), secret.to_string()))
}

fn exchange_code(request: &Request, binding: &ClientBinding) -> HttpJson<serde_json::Value> {
    let Some(code) = request.code.as_deref() else {
        return oauth_error(400, "invalid_request", "code is required");
    };
    let Some(redirect_uri) = request.redirect_uri.as_deref() else {
        return oauth_error(400, "invalid_request", "redirect_uri is required");
    };
    let Some(verifier) = request.code_verifier.as_deref() else {
        return oauth_error(400, "invalid_request", "code_verifier is required");
    };
    let code = match redeem_code(code, &binding.client_id, redirect_uri, verifier) {
        Ok(code) => code,
        Err(LookupError::Unavailable(error)) => {
            tracing::error!(%error, "authorization code store unavailable");
            return oauth_error(
                503,
                "temporarily_unavailable",
                "Authorization service is temporarily unavailable",
            );
        }
        Err(LookupError::BindingMismatch) => {
            return oauth_error(
                400,
                "invalid_grant",
                "Authorization code was issued to another client, redirect URI or verifier",
            );
        }
        Err(LookupError::Absent) => {
            return oauth_error(
                400,
                "invalid_grant",
                "Authorization code is invalid, expired, or already used",
            );
        }
    };
    if code.tenant_id != binding.tenant_id || code.application_id != binding.application_id {
        return oauth_error(
            400,
            "invalid_grant",
            "Authorization code binding is invalid",
        );
    }

    // The ID token first: if its signer (Cloud KMS) is unavailable, fail before a refresh token is stored.
    let id_token = match issue_id_token(
        &code.user_id,
        &code.client_id,
        &code.nonce,
        code.auth_time,
        code.authn.as_ref(),
    ) {
        Ok(token) => token,
        Err(error) => {
            tracing::error!(%error, "OIDC ID token issuance failed");
            return oauth_error(503, "temporarily_unavailable", "Token issuance unavailable");
        }
    };

    let scope = code.scopes.join(" ");
    let tokens = match issue_tokens_for_client(
        &code.user_id,
        &code.tenant_id,
        &code.application_id,
        &code.client_id,
        vec![],
        vec![],
        "user",
        None,
        &scope,
        code.authn.as_ref(),
    ) {
        Ok(tokens) => tokens,
        Err(error) => {
            tracing::error!(%error, "OIDC token issuance failed");
            return oauth_error(503, "temporarily_unavailable", "Token issuance unavailable");
        }
    };

    HttpJson::ok(serde_json::json!({
        "access_token": tokens.access_token,
        "token_type": "Bearer",
        "expires_in": tokens.expires_in,
        "refresh_token": tokens.refresh_token,
        "refresh_token_expires_in": tokens.refresh_expires_in,
        "id_token": id_token,
        "scope": tokens.scope,
    }))
}

fn exchange_refresh(request: &Request, binding: &ClientBinding) -> HttpJson<serde_json::Value> {
    let Some(refresh_token) = request.refresh_token.as_deref() else {
        return oauth_error(400, "invalid_request", "refresh_token is required");
    };
    match rotate_refresh_token_for_client(refresh_token, &binding.client_id) {
        RotationOutcome::Rotated {
            new_access_token,
            new_refresh_token,
            access_expires_in,
            refresh_expires_in,
            scope,
            ..
        } => HttpJson::ok(serde_json::json!({
            "access_token": new_access_token,
            "token_type": "Bearer",
            "expires_in": access_expires_in,
            "refresh_token": new_refresh_token,
            "refresh_token_expires_in": refresh_expires_in,
            "scope": scope,
        })),
        RotationOutcome::RedisUnavailable => oauth_error(
            503,
            "temporarily_unavailable",
            "Refresh service is temporarily unavailable",
        ),
        RotationOutcome::InvalidToken | RotationOutcome::ReuseDetected { .. } => {
            oauth_error(400, "invalid_grant", "Refresh token is invalid")
        }
    }
}

fn oauth_error(status: u16, error: &str, error_description: &str) -> HttpJson<serde_json::Value> {
    HttpJson::new(
        status,
        serde_json::json!({
            "error": error,
            "error_description": error_description,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::parse_basic;

    #[test]
    fn parses_basic_client_credentials() {
        assert_eq!(
            parse_basic("Basic Y2xpZW50OnNlY3JldA=="),
            Some(("client".to_string(), "secret".to_string()))
        );
        assert_eq!(parse_basic("Bearer token"), None);
    }

    // ---- client authentication: one lookup, and the same cost for every rejection of a secret

    use super::authenticate;
    use crate::services::client_registry::ClientBinding;
    use sesame_common::oidc_client::{ClientPolicy, ClientType, TokenEndpointAuthMethod};
    use std::cell::Cell;

    fn binding(client_type: ClientType, method: TokenEndpointAuthMethod) -> ClientBinding {
        ClientBinding {
            client_id: "web".into(),
            tenant_id: "t".into(),
            portal: "p".into(),
            application_id: "a".into(),
            authority_class: "c".into(),
            policy: ClientPolicy {
                client_type,
                token_endpoint_auth_method: method,
                pkce_s256_required: true,
                grants: vec!["authorization_code".into()],
                response_types: vec![],
                scopes: vec![],
                audiences: vec![],
            },
            redirect_uris: vec![],
            post_logout_redirect_uris: vec![],
            initiate_login_uri: None,
        }
    }

    /// (result, lookups, argon2 verifications) for one attempt; `known` resolves only client "web".
    fn attempt(
        basic: Option<(&str, &str)>,
        id: Option<&str>,
        secret: Option<&str>,
        known: Option<ClientBinding>,
    ) -> (bool, usize, usize) {
        let (lookups, hashes) = (Cell::new(0), Cell::new(0));
        let result = authenticate(
            basic,
            id,
            secret,
            |client_id| {
                lookups.set(lookups.get() + 1);
                known
                    .clone()
                    .filter(|_| client_id == "web")
                    .map(|b| (b, 7_u8))
            },
            |_, presented| {
                hashes.set(hashes.get() + 1);
                presented == "right"
            },
            |_| hashes.set(hashes.get() + 1),
        );
        (result.is_ok(), lookups.get(), hashes.get())
    }

    #[test]
    fn a_confidential_client_with_the_right_secret_is_accepted_with_one_lookup() {
        let c = Some(binding(
            ClientType::Confidential,
            TokenEndpointAuthMethod::ClientSecretBasic,
        ));
        assert_eq!(attempt(Some(("web", "right")), None, None, c), (true, 1, 1));
        let c = Some(binding(
            ClientType::Confidential,
            TokenEndpointAuthMethod::ClientSecretPost,
        ));
        assert_eq!(attempt(None, Some("web"), Some("right"), c), (true, 1, 1));
    }

    #[test]
    fn unknown_client_wrong_secret_and_inactive_client_cost_the_same() {
        let c = Some(binding(
            ClientType::Confidential,
            TokenEndpointAuthMethod::ClientSecretBasic,
        ));
        // wrong secret: one real verification
        assert_eq!(
            attempt(Some(("web", "wrong")), None, None, c.clone()),
            (false, 1, 1)
        );
        // unknown client id: one verification against the dummy hash
        assert_eq!(
            attempt(Some(("nobody", "wrong")), None, None, c.clone()),
            (false, 1, 1)
        );
        // a disabled client (or one of an inactive tenant) does not resolve: same as unknown
        assert_eq!(
            attempt(Some(("web", "right")), None, None, None),
            (false, 1, 1)
        );
        // the wrong method for the client: rejected, same cost
        let post = Some(binding(
            ClientType::Confidential,
            TokenEndpointAuthMethod::ClientSecretPost,
        ));
        assert_eq!(
            attempt(Some(("web", "right")), None, None, post),
            (false, 1, 1)
        );
        // a secret sent for a public client: rejected, same cost
        let public = Some(binding(ClientType::Public, TokenEndpointAuthMethod::None));
        assert_eq!(
            attempt(None, Some("web"), Some("x"), public.clone()),
            (false, 1, 1)
        );
        // a public client without a secret: accepted, no secret work at all
        assert_eq!(attempt(None, Some("web"), None, public), (true, 1, 0));
    }

    #[test]
    fn no_client_id_is_rejected_without_a_lookup() {
        assert_eq!(attempt(None, None, Some("x"), None), (false, 0, 1));
        assert_eq!(attempt(None, None, None, None), (false, 0, 0));
    }
}
