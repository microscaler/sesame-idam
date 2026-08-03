// BRRTRouter: user-owned

//! `POST /organizations/{org_id}/owner/transfer/challenge` — email OTP for product transfer.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_idam_org_mgmt_gen::handlers::challenge_org_owner_transfer::Request;
use uuid::Uuid;

use crate::services::{org_lifecycle, owner_transfer_otp};
use sesame_idam_org_mgmt::org_auth;

#[handler(ChallengeOrgOwnerTransferController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let (caller_id, tenant_id) =
        match org_auth::require_caller(&req.jwt_claims, req.data.x_tenant_id.as_deref()) {
            Ok(principal) => principal,
            Err(response) => return response,
        };

    let caller_uuid = match Uuid::parse_str(&caller_id) {
        Ok(id) => id,
        Err(_) => return org_auth::error_json(400, "validation_error", "Invalid user id"),
    };

    let exec = sesame_idam_database::db();
    match org_lifecycle::require_active_owner(exec, &tenant_id, &req.data.org_id, caller_uuid) {
        Ok(()) => {}
        Err(org_lifecycle::OrgLifecycleError::Forbidden) => {
            return org_auth::error_json(
                403,
                "forbidden",
                "Only the current organization Owner may request a transfer challenge",
            );
        }
        Err(org_lifecycle::OrgLifecycleError::NotFound) => {
            return org_auth::error_json(404, "not_found", "Organization not found");
        }
        Err(org_lifecycle::OrgLifecycleError::InvalidId(msg)) => {
            return org_auth::error_json(400, "validation_error", &msg);
        }
        Err(other) => {
            tracing::error!(error = %other, "owner transfer challenge auth failed");
            return org_auth::error_json(500, "internal_error", "An unexpected error occurred");
        }
    }

    let Some(email) = org_lifecycle::lookup_user_email(exec, &tenant_id, caller_uuid) else {
        tracing::error!(
            user_id = %caller_uuid,
            tenant = %tenant_id,
            "owner transfer challenge: caller email unavailable"
        );
        return org_auth::error_json(
            503,
            "otp_unavailable",
            "Unable to deliver a verification code for this account",
        );
    };

    let expires_in_secs = owner_transfer_otp::ttl_secs();
    let code = match owner_transfer_otp::create(&tenant_id, &req.data.org_id, &caller_id) {
        Ok(code) => code,
        Err(error) => {
            tracing::error!(%error, "owner transfer OTP mint failed");
            return org_auth::error_json(
                503,
                "otp_unavailable",
                "Verification code store is temporarily unavailable",
            );
        }
    };

    let body = format!(
        "Your Sesame ownership-transfer verification code is: {code}\n\n\
         It expires in {expires_in_secs} seconds. If you did not request a transfer, \
         secure your account and ignore this email."
    );
    if let Err(error) = sesame_common::smtp::send_email(
        &email,
        "Confirm organization ownership transfer",
        &body,
    ) {
        tracing::error!(%error, tenant = %tenant_id, "owner transfer OTP email failed");
        // Still return generic success so attackers cannot probe delivery; code remains valid.
    }

    HttpJson::new(
        200,
        serde_json::json!({
            "success": true,
            "expires_in_secs": expires_in_secs,
            "channel": "email",
        }),
    )
}
