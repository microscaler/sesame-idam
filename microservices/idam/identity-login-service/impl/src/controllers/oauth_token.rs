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
use crate::services::oidc_authorization::redeem_authorization_code;
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
    let client_id = basic
        .as_ref()
        .map(|(id, _)| id.as_str())
        .or(request.client_id.as_deref())
        .ok_or(())?
        .to_string();
    let binding = ClientRegistry::resolve_active(&client_id, exec).map_err(|_| ())?;

    match (
        &binding.policy.client_type,
        &binding.policy.token_endpoint_auth_method,
    ) {
        (ClientType::Public, TokenEndpointAuthMethod::None)
            if basic.is_none() && request.client_secret.is_none() =>
        {
            Ok(binding)
        }
        (ClientType::Confidential, TokenEndpointAuthMethod::ClientSecretBasic) => {
            let (_, secret) = basic.as_ref().ok_or(())?;
            ClientRegistry::authenticate_confidential(&client_id, secret, exec).map_err(|_| ())
        }
        (ClientType::Confidential, TokenEndpointAuthMethod::ClientSecretPost)
            if basic.is_none() =>
        {
            ClientRegistry::authenticate_confidential(
                &client_id,
                request.client_secret.as_deref().ok_or(())?,
                exec,
            )
            .map_err(|_| ())
        }
        _ => Err(()),
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
    let Some(code) = redeem_authorization_code(code, &binding.client_id, redirect_uri, verifier)
    else {
        return oauth_error(400, "invalid_grant", "Authorization code is invalid");
    };
    if code.tenant_id != binding.tenant_id || code.application_id != binding.application_id {
        return oauth_error(
            400,
            "invalid_grant",
            "Authorization code binding is invalid",
        );
    }

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
    ) {
        Ok(tokens) => tokens,
        Err(error) => {
            tracing::error!(%error, "OIDC token issuance failed");
            return oauth_error(503, "temporarily_unavailable", "Token issuance unavailable");
        }
    };
    let id_token = match issue_id_token(&code.user_id, &code.client_id, &code.nonce, code.auth_time)
    {
        Ok(token) => token,
        Err(error) => {
            tracing::error!(%error, "OIDC ID token issuance failed");
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
}
