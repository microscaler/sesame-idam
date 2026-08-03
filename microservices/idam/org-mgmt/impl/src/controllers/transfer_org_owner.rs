// BRRTRouter: user-owned

//! `POST /organizations/{org_id}/owner/transfer` — Owner-initiated succession.

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_common::VersionStore;
use sesame_idam_org_mgmt_gen::handlers::transfer_org_owner::Request;
use uuid::Uuid;

use crate::services::org_lifecycle::{
    self, FormerOwnerDisposition, OrgLifecycleError, TransferActor,
};
use crate::services::{owner_transfer_otp, password_verify};
use sesame_idam_org_mgmt::org_auth;

#[handler(TransferOrgOwnerController)]
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

    // Dual-factor: password (knowledge) + email OTP (out-of-band). Email alone is
    // insufficient when a walk-away attacker can also open the Owner's webmail.
    let password = req.data.password.as_deref().map(str::trim).unwrap_or("");
    if password.is_empty() {
        return org_auth::error_json(
            403,
            "password_required",
            "Current account password is required to transfer ownership",
        );
    }
    let exec = sesame_idam_database::db();
    if !password_verify::verify_caller_password(exec, &tenant_id, caller_uuid, password) {
        return org_auth::error_json(
            403,
            "password_invalid",
            "Password is incorrect",
        );
    }

    let otp = req.data.otp.as_deref().map(str::trim).unwrap_or("");
    if otp.is_empty() {
        return org_auth::error_json(
            403,
            "otp_required",
            "Email verification code is required to transfer ownership",
        );
    }
    if !owner_transfer_otp::verify_and_consume(&tenant_id, &req.data.org_id, &caller_id, otp) {
        return org_auth::error_json(
            403,
            "otp_invalid",
            "Verification code is invalid or expired",
        );
    }

    let disposition = match FormerOwnerDisposition::parse(req.data.former_owner_disposition.as_deref())
    {
        Ok(d) => d,
        Err(OrgLifecycleError::InvalidId(msg)) => {
            return org_auth::error_json(400, "validation_error", &msg);
        }
        Err(_) => {
            return org_auth::error_json(400, "validation_error", "Invalid disposition");
        }
    };

    match org_lifecycle::transfer_owner(
        exec,
        &tenant_id,
        &req.data.org_id,
        TransferActor::ProductOwner {
            caller_user_id: caller_uuid,
        },
        &req.data.successor_user_id,
        req.data.from_user_id.as_deref(),
        disposition,
    ) {
        Ok(result) => finish_transfer(result, "product_owner", req.data.reason.as_deref(), req.data.ticket_id.as_deref()),
        Err(err) => map_transfer_error(err),
    }
}

pub(crate) fn finish_transfer(
    result: org_lifecycle::OwnerTransferResult,
    actor_type: &str,
    reason: Option<&str>,
    ticket_id: Option<&str>,
) -> HttpJson<serde_json::Value> {
    let former = result.former_owner_user_id.to_string();
    let successor = result.successor_user_id.to_string();

    if let Err(error) = VersionStore::from_env().and_then(|store| {
        store.increment_subject(&former)?;
        store.increment_subject(&successor)?;
        Ok(())
    }) {
        tracing::error!(%error, %former, %successor, "token version bump failed after owner transfer");
        return org_auth::error_json(
            503,
            "security_state_unavailable",
            "Session invalidation is temporarily unavailable",
        );
    }

    tracing::info!(
        org_id = %result.org_id,
        former_owner_user_id = %former,
        successor_user_id = %successor,
        disposition = result.former_owner_disposition.as_str(),
        actor_type,
        reason = reason.unwrap_or(""),
        ticket_id = ticket_id.unwrap_or(""),
        "org.owner.transferred"
    );

    HttpJson::new(
        200,
        serde_json::json!({
            "org_id": result.org_id.to_string(),
            "former_owner_user_id": former,
            "successor_user_id": successor,
            "former_owner_disposition": result.former_owner_disposition.as_str(),
        }),
    )
}

pub(crate) fn map_transfer_error(err: OrgLifecycleError) -> HttpJson<serde_json::Value> {
    match err {
        OrgLifecycleError::Forbidden => org_auth::error_json(
            403,
            "forbidden",
            "Only the current organization Owner may transfer ownership on the product path",
        ),
        OrgLifecycleError::SuccessorNotMember => org_auth::error_json(
            404,
            "successor_not_member",
            "Successor is not an active member of the organization",
        ),
        OrgLifecycleError::AmbiguousOwner => org_auth::error_json(
            400,
            "ambiguous_owner",
            "Multiple owners exist; from_user_id is required",
        ),
        OrgLifecycleError::OwnerNotFound => {
            org_auth::error_json(404, "owner_not_found", "Specified owner was not found")
        }
        OrgLifecycleError::NotFound => {
            org_auth::error_json(404, "not_found", "Organization not found")
        }
        OrgLifecycleError::InvalidId(msg) => {
            org_auth::error_json(400, "validation_error", &msg)
        }
        other => {
            tracing::error!(error = %other, "owner transfer failed");
            org_auth::error_json(500, "internal_error", "An unexpected error occurred")
        }
    }
}
