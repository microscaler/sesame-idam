// BRRTRouter: user-owned

//! `PUT /tenant/sms/{environment}` — a tenant configures its own SMS sender.
//!
//! Tenant self-service (ADR-011). The tenant comes from the bearer token, so
//! this handler cannot be pointed at another tenant's configuration: there is
//! no identifier in the request to change.
//!
//! Everything below the authority check is shared with the platform-scoped
//! variant, which is the point — an operator acting on a tenant during a
//! support escalation and the tenant acting on itself must produce the same
//! result, or the two surfaces will drift apart and one will grow a bug the
//! other does not have.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_identity_login_service_gen::handlers::tenant_sms_upsert::Request;

use crate::controllers::platform_tenant_sms_upsert::{
    apply_sms_config, envelope_custody_allowed, forbidden_envelope_custody, internal_error,
    read_back, SmsUpsertInput,
};
use crate::models::tenant_sms_config::CUSTODY_ENVELOPE;
use crate::services::tenant_admin::tenant_admin_principal;

#[handler(TenantSmsUpsertController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let admin = match tenant_admin_principal(&req.jwt_claims) {
        Ok(a) => a,
        Err(resp) => return resp,
    };
    let environment = req.data.environment.trim();
    let custody = req.data.custody_mode.trim();

    // Envelope custody means Sesame holds the tenant's credential. A tenant
    // cannot grant itself that privilege by asking — the allow-list is
    // platform-side, and Connect is the answer for everyone else.
    if custody == CUSTODY_ENVELOPE && !envelope_custody_allowed(&admin.tenant) {
        tracing::warn!(
            tenant = %admin.tenant,
            "tenant requested envelope custody without being allow-listed"
        );
        return forbidden_envelope_custody();
    }

    let exec = sesame_idam_database::db();
    let input = SmsUpsertInput {
        custody_mode: custody,
        connected_account_sid: req.data.connected_account_sid.as_deref(),
        account_sid: req.data.account_sid.as_deref(),
        auth_token: req.data.auth_token.as_deref(),
        messaging_service_sid: req.data.messaging_service_sid.as_deref(),
        from_number: req.data.from_number.as_deref(),
        campaign_ref: req.data.campaign_ref.as_deref(),
        daily_spend_ceiling_cents: req.data.daily_spend_ceiling_cents,
    };

    match apply_sms_config(&admin.tenant, environment, &input, exec) {
        Ok(()) => read_back(&admin.tenant, environment, exec),
        Err(resp) => resp,
    }
}
