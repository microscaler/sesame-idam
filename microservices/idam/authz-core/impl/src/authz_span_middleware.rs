//! Authz request tracing middleware — wraps every incoming request with
//! an `authz.request` span and records whether the request was allowed
//! or denied based on the response status.

use std::time::Duration;

use brrtrouter::dispatcher::{HandlerRequest, HandlerResponse};
use brrtrouter::middleware::Middleware;

pub struct AuthzSpanMiddleware;

impl AuthzSpanMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Middleware for AuthzSpanMiddleware {
    fn before(&self, req: &HandlerRequest) -> Option<HandlerResponse> {
        let span = tracing::span!(
            tracing::Level::INFO,
            "authz.request",
            route = req.path,
            method = %req.method
        );
        let _guard = span.enter();
        tracing::debug!(route = req.path, method = %req.method, "authz request started");
        None
    }

    fn after(&self, req: &HandlerRequest, res: &mut HandlerResponse, _latency: Duration) {
        let status: u16 = res.status;
        let allowed = status < 400;

        tracing::Span::current().record("result", if allowed { "allowed" } else { "denied" });

        if !allowed {
            tracing::Span::current().record("status", status);
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
