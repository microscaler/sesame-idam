// BRRTRouter: user-owned

//! `GET /identity/me` — current user profile (DB-backed).
//!
//! Identity comes from validated JWT claims on [`TypedHandlerRequest::jwt_claims`]
//! (BR-2). PII lives here by design: it was removed from access tokens (Story 2.3).

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_session_service_gen::handlers::users_me_get::Request;

use crate::auth_context::authenticated_principal;
use crate::models::user::UserModel;
use crate::models::user_profile::UserProfileModel;
use crate::tenant_db::{load_profile, ProfileLoad};

/// Build the spec `UserProfile` JSON from the user row + optional profile.
pub fn profile_json(user: &UserModel, profile: Option<&UserProfileModel>) -> serde_json::Value {
    let first_name = profile.and_then(|p| p.first_name.clone());
    let last_name = profile.and_then(|p| p.last_name.clone());
    let name = match (&first_name, &last_name) {
        (Some(f), Some(l)) => Some(format!("{f} {l}")),
        (Some(f), None) => Some(f.clone()),
        (None, Some(l)) => Some(l.clone()),
        (None, None) => None,
    };

    serde_json::json!({
        "sub": user.id,
        "user_id": user.id,
        "email": user.email,
        "email_verified": user.email_verified,
        "phone": user.phone,
        "phone_verified": user.phone_verified,
        "name": name,
        "first_name": first_name,
        "last_name": last_name,
        "avatar_url": profile.and_then(|p| p.avatar_url.clone()),
        "is_active": user.status == "active",
        "created_at": user.created_at.to_rfc3339(),
        "updated_at": user.updated_at.to_rfc3339(),
    })
}

#[handler(UsersMeGetController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    use crate::audit::EMITTER;
    use sesame_common::audit::{AuditEventType, AuditLogEntry};

    let (user_id, tenant_id) =
        match authenticated_principal(&req.jwt_claims, req.data.x_tenant_id.as_deref()) {
            Ok(principal) => principal,
            Err(response) => return response,
        };

    let entry = AuditLogEntry::new(AuditEventType::JwtValidated, "identity-session-service")
        .tenant_id(tenant_id.clone())
        .user_id(user_id.to_string())
        .decision_source("users_me_get")
        .result("allowed")
        .build();
    if let Ok(entry) = entry {
        EMITTER.emit(entry);
    }

    let (user, profile) = match load_profile(&tenant_id, user_id) {
        // A disabled account's access token is refused here too (the token status check already refuses it once the
        // disable bumped the subject version; this holds even if that bump was missed).
        Ok(ProfileLoad::Found(user, _)) if user.status != "active" => {
            return HttpJson::new(
                401,
                serde_json::json!({
                    "error": "invalid_request",
                    "error_description": "Unauthorized (invalid or missing token)",
                }),
            );
        }
        Ok(ProfileLoad::Found(user, profile)) => (user, profile),
        Ok(ProfileLoad::NotFound) => {
            return HttpJson::new(
                401,
                serde_json::json!({
                    "error": "invalid_request",
                    "error_description": "Unauthorized (invalid or missing token)",
                }),
            );
        }
        Err(e) => {
            tracing::error!(error = %e, "users_me_get: user lookup failed");
            return internal_error();
        }
    };

    HttpJson::ok(profile_json(&user, profile.as_ref()))
}

fn internal_error() -> HttpJson<serde_json::Value> {
    HttpJson::new(
        500,
        serde_json::json!({
            "error": "internal_error",
            "error_description": "An unexpected error occurred",
        }),
    )
}

/// Contract tests: the hand-built bodies against the `users_me_get` response schemas in the
/// spec this service serves (`gen/doc/openapi.yaml`), as the server's response validation
/// checks them (`brrtrouter::contract`).
#[cfg(test)]
mod contract_tests {
    use super::*;
    use brrtrouter::contract::ResponseContract;
    use std::sync::OnceLock;

    fn contract() -> &'static ResponseContract {
        static C: OnceLock<ResponseContract> = OnceLock::new();
        C.get_or_init(|| {
            ResponseContract::load(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../gen/doc/openapi.yaml"
            ))
            .expect("identity-session spec")
        })
    }

    fn user(phone: Option<&str>) -> UserModel {
        let t = chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 10, 26, 12, 0, 0).unwrap();
        UserModel {
            id: uuid::Uuid::new_v4(),
            email: "ann@example.com".into(),
            password_hash: String::new(),
            tenant_id: "acme".into(),
            status: "active".into(),
            email_verified: true,
            phone: phone.map(str::to_string),
            phone_verified: false,
            created_at: t,
            updated_at: t,
        }
    }

    fn profile(first: Option<&str>, last: Option<&str>) -> UserProfileModel {
        let t = chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 10, 26, 12, 0, 0).unwrap();
        UserProfileModel {
            id: uuid::Uuid::new_v4(),
            user_id: uuid::Uuid::new_v4(),
            first_name: first.map(str::to_string),
            last_name: last.map(str::to_string),
            avatar_url: Some("https://example.com/a.png".into()),
            created_at: t,
            updated_at: t,
        }
    }

    /// Every shape `profile_json` produces (with and without a profile row, names partly
    /// set, no phone) matches the 200 schema's types.
    #[test]
    fn profile_json_matches_the_200_schema() {
        let c = contract();
        for (u, p) in [
            (
                user(Some("+14155551234")),
                Some(profile(Some("Ann"), Some("Lee"))),
            ),
            (user(None), Some(profile(Some("Ann"), None))),
            (user(None), Some(profile(None, None))),
            (user(None), None),
        ] {
            c.assert_valid("users_me_get", 200, &profile_json(&u, p.as_ref()));
        }
    }

    /// Fields the body sends that the `UserProfile` schema does not declare. The schema does
    /// not forbid extra keys, so these pass validation; listed here so a rename on either side
    /// shows up as a test change.
    #[test]
    fn profile_json_fields_the_schema_does_not_declare() {
        let body = profile_json(
            &user(Some("+14155551234")),
            Some(&profile(Some("Ann"), None)),
        );
        let mut extra = contract()
            .undeclared_fields("users_me_get", 200, &body)
            .expect("200 schema");
        extra.sort();
        // `phone` is the spec's `phone_number`; `is_active` and `created_at` are not in the schema.
        assert_eq!(extra, vec!["/created_at", "/is_active", "/phone"]);
    }

    #[test]
    fn error_bodies_match_their_schemas() {
        let c = contract();
        c.assert_valid(
            "users_me_get",
            401,
            &serde_json::json!({
                "error": "invalid_request",
                "error_description": "Unauthorized (invalid or missing token)",
            }),
        );
        let e = internal_error();
        c.assert_valid("users_me_get", e.status, &e.body);
    }
}
