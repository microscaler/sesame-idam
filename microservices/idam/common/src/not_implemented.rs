//! The base layer of route registration: every operation in the spec answers `501 Not Implemented` until an impl
//! controller replaces it.
//!
//! BRRTRouter's generated `register_from_spec` wires the gen crate's example controllers, which answer with the
//! spec's example payloads: fine for a mock server, wrong for a deployment, where an unimplemented `authorize` would
//! answer "allowed" and an unimplemented listing would return invented records. Services register this layer first
//! and their impl controllers on top (BRRTRouter ADR 0001 register-and-overwrite, with an honest base layer).

use brrtrouter::dispatcher::{spawn_untyped_with_stack_size_and_name, Dispatcher, HandlerRequest, HandlerResponse};
use brrtrouter::spec::RouteMeta;

/// Body of every `501` answer.
#[must_use]
pub fn body(handler: &str) -> serde_json::Value {
    serde_json::json!({
        "error": "not_implemented",
        "message": format!("{handler} is not implemented"),
    })
}

fn answer(req: HandlerRequest) -> HandlerResponse {
    HandlerResponse::json(501, body(&req.handler_name))
}

/// Route every operation in `routes` to one coroutine that answers `501`. Register impl controllers afterwards; each
/// `add_route` replaces the entry for its route.
///
/// # Errors
///
/// When the coroutine cannot be spawned.
///
/// # Safety
///
/// Spawns a coroutine: the `may` runtime must be configured, as for any BRRTRouter handler.
pub unsafe fn register_not_implemented(
    dispatcher: &mut Dispatcher,
    routes: &[RouteMeta],
) -> std::io::Result<()> {
    let tx = spawn_untyped_with_stack_size_and_name(answer, 16 * 1024, Some("not_implemented"))?;
    for route in routes {
        dispatcher.add_route(route.clone(), tx.clone());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_names_the_operation_only() {
        let b = body("authorize");
        assert_eq!(b["error"], "not_implemented");
        assert_eq!(b["message"], "authorize is not implemented");
    }
}
