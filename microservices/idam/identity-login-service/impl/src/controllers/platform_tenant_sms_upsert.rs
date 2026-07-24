// BRRTRouter: user-owned

//! `PUT /platform/tenants/{slug}/sms/{environment}` — upsert the tenant's SMS
//! sender configuration (ADR-009 §7).
//!
//! # Custody, and why `connect` is the default answer
//!
//! `connect` stores a revocable connected AccountSid and nothing else: Twilio
//! bills the tenant directly and Sesame never holds a secret it could leak.
//! `envelope` accepts the tenant's own auth token — strictly better for the
//! tenant's convenience and strictly worse for our liability — so it is
//! gated to an explicit dogfood allow-list (`SMS_ENVELOPE_CUSTODY_TENANTS`)
//! rather than being available to anyone who asks.
//!
//! # Nothing is trusted on arrival
//!
//! A newly supplied credential lands as `pending_validation`, and only an
//! `active` config resolves for sending. A typo therefore fails closed to
//! email instead of silently burning sends against a broken account.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::platform_tenant_sms_upsert::Request;

use crate::models::tenant_sms_config::{TenantSmsConfigModel, CUSTODY_CONNECT, CUSTODY_ENVELOPE};
use crate::services::tenant_service::{TenantService, STATUS_DEPROVISIONED};
use crate::services::tenant_sms_service::{SmsConfigInput, SmsConfigView, TenantSmsService};

/// Tenants permitted to hand Sesame a raw credential. Empty (the default)
/// means nobody — external tenants must use Twilio Connect.
fn envelope_custody_allowed(tenant: &str) -> bool {
    std::env::var("SMS_ENVELOPE_CUSTODY_TENANTS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .any(|allowed| allowed == tenant)
}

#[handler(PlatformTenantSmsUpsertController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let slug = req.data.slug.trim();
    let environment = req.data.environment.trim();
    let custody = req.data.custody_mode.trim();
    let exec = sesame_idam_database::db();

    match TenantService::find_by_slug(slug, exec) {
        Ok(Some(t)) if t.status != STATUS_DEPROVISIONED => {}
        Ok(_) => return not_found(),
        Err(e) => {
            tracing::error!(error = %e, tenant = slug, "sms upsert: tenant lookup failed");
            return internal_error();
        }
    }

    let opts = SmsConfigInput {
        messaging_service_sid: non_empty(req.data.messaging_service_sid.as_deref()),
        from_number: non_empty(req.data.from_number.as_deref()),
        campaign_ref: non_empty(req.data.campaign_ref.as_deref()),
        daily_spend_ceiling_cents: req.data.daily_spend_ceiling_cents,
    };

    let stored = match custody {
        CUSTODY_CONNECT => {
            let Some(sid) = non_empty(req.data.connected_account_sid.as_deref()) else {
                return bad_request("connected_account_sid is required for connect custody");
            };
            TenantSmsService::upsert_connect(slug, environment, &sid, &opts, exec)
        }
        CUSTODY_ENVELOPE => {
            if !envelope_custody_allowed(slug) {
                tracing::warn!(
                    tenant = slug,
                    "refused envelope custody — tenant not on the dogfood allow-list"
                );
                return HttpJson::new(
                    403,
                    serde_json::json!({
                        "error": "envelope_custody_forbidden",
                        "error_description":
                            "This tenant must use Twilio Connect; Sesame will not store its credentials."
                    }),
                );
            }
            let Some(account_sid) = non_empty(req.data.account_sid.as_deref()) else {
                return bad_request("account_sid is required for envelope custody");
            };
            TenantSmsService::upsert_envelope(
                slug,
                environment,
                &account_sid,
                non_empty(req.data.auth_token.as_deref()).as_deref(),
                &opts,
                exec,
            )
        }
        other => {
            return bad_request(&format!(
                "unsupported custody_mode '{other}' (expected 'connect' or 'envelope')"
            ));
        }
    };

    if let Err(e) = stored {
        // The message may mention the credential's shape but never its value.
        tracing::error!(error = %e, tenant = slug, environment, "sms config write failed");
        let msg = e.to_string();
        if msg.contains("auth_token is required") {
            return bad_request("auth_token is required when no credential is stored");
        }
        return internal_error();
    }

    match TenantSmsService::find(slug, environment, exec) {
        Ok(Some(config)) => view_json(&config),
        Ok(None) => internal_error(),
        Err(e) => {
            tracing::error!(error = %e, "sms config read-back failed");
            internal_error()
        }
    }
}

fn non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
}

/// Serialise the secret-free view. Shared by all three SMS controllers so the
/// response shape can only be defined in one place.
pub(crate) fn view_json(config: &TenantSmsConfigModel) -> HttpJson<serde_json::Value> {
    let view = SmsConfigView::from(config);
    serde_json::to_value(&view).map_or_else(
        |e| {
            tracing::error!(error = %e, "sms config serialisation failed");
            internal_error()
        },
        HttpJson::ok,
    )
}

pub(crate) fn not_found() -> HttpJson<serde_json::Value> {
    HttpJson::new(
        404,
        serde_json::json!({
            "error": "not_found",
            "error_description": "No SMS configuration for that tenant and environment."
        }),
    )
}

fn bad_request(description: &str) -> HttpJson<serde_json::Value> {
    HttpJson::new(
        400,
        serde_json::json!({
            "error": "invalid_request",
            "error_description": description
        }),
    )
}

pub(crate) fn internal_error() -> HttpJson<serde_json::Value> {
    HttpJson::new(
        500,
        serde_json::json!({
            "error": "internal_error",
            "error_description": "internal_error"
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// The allow-list is the whole guard: absent config must mean "no tenant
    /// may hand us a credential", never "every tenant may".
    #[test]
    fn envelope_custody_is_denied_by_default() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::env::remove_var("SMS_ENVELOPE_CUSTODY_TENANTS");
        assert!(!envelope_custody_allowed("hauliage"));
        std::env::set_var("SMS_ENVELOPE_CUSTODY_TENANTS", "");
        assert!(!envelope_custody_allowed("hauliage"));
    }

    #[test]
    fn envelope_custody_allows_only_listed_tenants() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::env::set_var("SMS_ENVELOPE_CUSTODY_TENANTS", "hauliage, pricewhisperer");
        assert!(envelope_custody_allowed("hauliage"));
        assert!(envelope_custody_allowed("pricewhisperer"));
        assert!(!envelope_custody_allowed("someone-else"));
        // Substring matches must not slip through.
        assert!(!envelope_custody_allowed("haul"));
        assert!(!envelope_custody_allowed("hauliage-evil"));
        std::env::remove_var("SMS_ENVELOPE_CUSTODY_TENANTS");
    }

    #[test]
    fn blank_fields_are_treated_as_absent() {
        assert_eq!(non_empty(Some("  ")), None);
        assert_eq!(non_empty(Some(" AC123 ")), Some("AC123".to_string()));
        assert_eq!(non_empty(None), None);
    }
}
