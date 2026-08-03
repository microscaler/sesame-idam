//! Access-token claims → verified-principal JSON (Epic 15.2).
//!
//! Mapping rules are normative in
//! `docs/standards-first-oidc/verified-principal-mapping-v1.md`.

use serde_json::{json, Value};

use super::types::AccessClaims;

/// Profile version embedded in [`verified-principal-v1.schema.json`].
pub const VERIFIED_PRINCIPAL_PROFILE_VERSION: &str = "1.0.0";

/// Map validated [`AccessClaims`] into verified-principal v1 JSON.
///
/// Callers must only pass claims produced after cryptographic verification
/// (for example [`super::verify_access_token`]).
pub fn map_access_claims_to_verified_principal(claims: &AccessClaims) -> Value {
    let mut roles = claims.sx.roles.clone();
    roles.sort();
    roles.dedup();
    let mut permissions = claims.sx.permissions.clone();
    permissions.sort();
    permissions.dedup();

    let mut principal = json!({
        "profile_version": VERIFIED_PRINCIPAL_PROFILE_VERSION,
        "tenant_id": claims.tenant_id,
        "subject": claims.sub,
        "client_id": claims.client_id,
        "application_id": claims.client_id,
        "session_id": claims.sid,
        "token_version": claims.ver,
        "organization_id": claims.org_id,
        "user_type": claims.user_type,
        "roles": roles,
        "permissions": permissions,
        "entitlements_ref": claims.sx.entitlements_ref,
        "entitlements_hash": claims.sx.entitlements_hash,
        "actor": claims.act.as_ref().map(|act| json!({
            "sub": act.sub,
        })),
    });

    if !claims.sx.portal.is_empty() {
        principal["portal"] = Value::String(claims.sx.portal.clone());
    }

    principal
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jwt::builders::{AccessClaimsBuilder, SesameAuthzClaimsBuilder};
    use crate::jwt::signer::Ed25519Signer;
    use crate::jwt::verify_access_token;

    fn sample_claims(with_org: bool) -> AccessClaims {
        let now = chrono::Utc::now().timestamp();
        let sx = SesameAuthzClaimsBuilder::new()
            .tenant("acme")
            .portal("acme-web")
            .roles(vec!["owner".into(), "owner".into()])
            .permissions(vec!["org:admin".into()])
            .build()
            .expect("sx");
        let mut builder = AccessClaimsBuilder::new()
            .iss("https://idam.example.com")
            .sub("11111111-1111-1111-1111-111111111111")
            .aud(vec!["identity-login".into()])
            .client_id("acme-web")
            .scope("openid profile")
            .exp(now + 300)
            .nbf(now - 5)
            .iat(now)
            .jti("jti-vp-1")
            .ver(1)
            .sid("sid-vp-1")
            .tenant_id("acme")
            .user_id("11111111-1111-1111-1111-111111111111")
            .user_type("customer")
            .sx(sx);
        if with_org {
            builder = builder.org_id("22222222-2222-2222-2222-222222222222");
        }
        builder.build().expect("claims")
    }

    #[test]
    fn maps_pre_org_principal_with_null_organization() {
        let principal = map_access_claims_to_verified_principal(&sample_claims(false));
        assert_eq!(principal["profile_version"], "1.0.0");
        assert_eq!(principal["subject"], "11111111-1111-1111-1111-111111111111");
        assert_eq!(principal["session_id"], "sid-vp-1");
        assert_eq!(principal["application_id"], "acme-web");
        assert_eq!(principal["portal"], "acme-web");
        assert_eq!(principal["organization_id"], Value::Null);
        assert_eq!(principal["roles"], json!(["owner"]));
        assert_eq!(principal["permissions"], json!(["org:admin"]));
    }

    #[test]
    fn maps_org_and_validates_against_minted_token() {
        let signer = Ed25519Signer::from_env_or_generate().expect("signer");
        let claims = sample_claims(true);
        let token = signer
            .sign_access_claims(&claims)
            .expect("sign access token");
        let verified = verify_access_token(&signer, &token, Some("acme")).expect("verify");
        let principal = map_access_claims_to_verified_principal(&verified);

        for key in [
            "profile_version",
            "tenant_id",
            "subject",
            "client_id",
            "application_id",
            "session_id",
            "token_version",
            "user_type",
            "roles",
            "permissions",
        ] {
            assert!(principal.get(key).is_some(), "missing required field {key}");
        }
        assert_eq!(
            principal["organization_id"],
            "22222222-2222-2222-2222-222222222222"
        );
        assert_eq!(principal["token_version"], 1);
    }
}
