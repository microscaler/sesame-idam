//! Authz request tracing middleware — wraps every incoming request with
//! an `authz.request` span and records whether the request was allowed
//! or denied based on the response status.
//!
//! This middleware runs at the dispatcher level (before route resolution),
//! so it captures all incoming requests regardless of handler.

use std::time::Duration;

use brrtrouter::dispatcher::{HandlerRequest, HandlerResponse};
use brrtrouter::middleware::Middleware;

/// `BRRTRouter` middleware that creates a `tracing` span for every
/// incoming request and records whether it was allowed or denied.
///
/// The span is named `authz.request` and includes `route`, `method`,
/// and `result` fields. If the response status is ≥ 400, the `status`
/// field is also recorded.
pub struct AuthzSpanMiddleware;

impl AuthzSpanMiddleware {
    /// Create a new `AuthzSpanMiddleware` instance.
    ///
    /// This is a stateless middleware, so the returned instance can be
    /// shared across all requests via `Arc`.
    pub fn new() -> Self {
        Self
    }
}

impl Middleware for AuthzSpanMiddleware {
    fn before(&self, req: &HandlerRequest) -> Option<HandlerResponse> {
        tracing::debug!(route = req.path, method = %req.method, "authz request started");
        None
    }

    // The span is made here, once the outcome is known. It used to be entered in `before()` and
    // read back through `Span::current()` here, but the guard dropped at the end of `before()` and on
    // may `Span::current()` is the worker thread's, not the request's (may_tracing ADR-0001), so the
    // result was recorded on whatever span that thread had, if any.
    fn after(&self, req: &HandlerRequest, res: &mut HandlerResponse, _latency: Duration) {
        let status: u16 = res.status;
        let allowed = status < 400;

        let span = tracing::span!(
            parent: None,
            tracing::Level::INFO,
            "authz.request",
            route = req.path,
            method = %req.method,
            result = if allowed { "allowed" } else { "denied" },
            status = tracing::field::Empty
        );
        if !allowed {
            span.record("status", status);
        }

        tracing::debug!(
            route = req.path,
            method = %req.method,
            status,
            result = if allowed { "allowed" } else { "denied" },
            "authz request completed"
        );
    }
}
