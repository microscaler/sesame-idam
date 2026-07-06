//! Raw (untyped) handler registration.
//!
//! `BRRTRouter`'s typed dispatch (`TypedHandlerRequest<T>`) drops
//! `HandlerRequest::jwt_claims` during conversion, so endpoints whose
//! behaviour depends on the authenticated principal (`/identity/me`,
//! userinfo) register a raw handler instead: the closure receives the full
//! `HandlerRequest` — including the claims the security provider validated —
//! and replies via `reply_tx`.

use brrtrouter::dispatcher::{HandlerRequest, HandlerResponse, HandlerSender};

/// Spawn a raw handler coroutine and return its dispatch sender.
///
/// Mirrors `brrtrouter::typed::spawn_typed_with_stack_size_and_name`
/// (dedicated coroutine, per-request panic recovery → 500).
///
/// # Panics
///
/// Panics if the coroutine cannot be spawned (startup-time only, same
/// policy as `BRRTRouter`'s typed spawn).
pub fn spawn_raw_handler<F>(name: &str, stack_size: usize, handler: F) -> HandlerSender
where
    F: Fn(&HandlerRequest) -> HandlerResponse + Send + 'static,
{
    let (tx, rx) = may::sync::mpsc::channel::<HandlerRequest>();

    let builder = may::coroutine::Builder::new()
        .name(format!("raw:{name}"))
        .stack_size(stack_size);

    // SAFETY: same contract as BRRTRouter's typed handler spawn — the
    // closure owns its captures and communicates only via channels.
    let spawn_result = unsafe {
        builder.spawn(move || {
            for req in &rx {
                let reply_tx = req.reply_tx.clone();
                let request_id = req.request_id;

                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handler(&req)));

                let response = match result {
                    Ok(resp) => resp,
                    Err(panic) => {
                        eprintln!("raw handler panicked: {panic:?}");
                        HandlerResponse::json(
                            500,
                            serde_json::json!({
                                "error": "internal_error",
                                "error_description": "An unexpected error occurred",
                                "request_id": request_id.to_string(),
                            }),
                        )
                    }
                };
                let _ = reply_tx.send(response);
            }
        })
    };

    #[allow(clippy::panic)]
    match spawn_result {
        Ok(_) => tx,
        Err(e) => panic!("failed to spawn raw handler coroutine: {e}"),
    }
}

/// Extract the authenticated principal (sub) and tenant from validated JWT
/// claims, cross-checked against the `X-Tenant-ID` header (HACK-401: both
/// locations must agree).
///
/// Returns `Err(HandlerResponse)` with the appropriate 401 when the request
/// is unauthenticated or the tenant does not match.
pub fn authenticated_principal(
    req: &HandlerRequest,
) -> Result<(uuid::Uuid, String), Box<HandlerResponse>> {
    let unauthorized = |desc: &str| {
        Box::new(HandlerResponse::json(
            401,
            serde_json::json!({
                "error": "invalid_request",
                "error_description": desc,
            }),
        ))
    };

    let Some(claims) = &req.jwt_claims else {
        return Err(unauthorized("Unauthorized (invalid or missing token)"));
    };

    let Some(sub) = claims.get("sub").and_then(|v| v.as_str()) else {
        return Err(unauthorized("Token missing sub claim"));
    };
    let Ok(user_id) = sub.parse::<uuid::Uuid>() else {
        return Err(unauthorized("Token sub is not a valid user id"));
    };

    let Some(tenant_id) = claims.get("tenant_id").and_then(|v| v.as_str()) else {
        return Err(unauthorized("Token missing tenant_id claim"));
    };

    // X-Tenant-ID header must agree with the token's tenant claim.
    match req.get_header("x-tenant-id") {
        Some(header_tenant) if header_tenant == tenant_id => {}
        Some(_) => {
            return Err(unauthorized("X-Tenant-ID does not match token tenant"));
        }
        None => {
            return Err(unauthorized("Missing required X-Tenant-ID header"));
        }
    }

    Ok((user_id, tenant_id.to_string()))
}
