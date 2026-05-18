// Middleware: adds Cache-Control, X-Content-Type-Options, Vary headers
// to the JWKS endpoint (/.well-known/jwks.json).
//
// BRRTRouter convention: response headers are injected via middleware `after()`
// hook, NOT in controllers/handlers. The handler returns a typed Response
// struct (pure data). Headers are an HTTP protocol concern handled by middleware.

use brrtrouter::dispatcher::HandlerRequest;
use brrtrouter::dispatcher::HandlerResponse;
use brrtrouter::middleware::Middleware;
use std::time::Duration;

/// Cache-Control header for JWKS responses.
/// Consumers should cache JWKS for 5 minutes to avoid excessive fetches.
const CACHE_CONTROL: &str = "public, max-age=300";

/// Prevent MIME sniffing on JWKS responses.
const X_CONTENT_TYPE_OPTIONS: &str = "nosniff";

/// Ensure CDN/proxy caching respects content negotiation.
const VARY: &str = "Accept";

/// Middleware that injects caching and security headers onto the JWKS endpoint.
pub struct JwksHeadersMiddleware;

impl Middleware for JwksHeadersMiddleware {
    fn after(&self, req: &HandlerRequest, res: &mut HandlerResponse, _latency: Duration) {
        // Only apply to the JWKS endpoint path.
        // Use ends_with for prefix safety (handles /, /v1/, /api/v1/, etc.)
        // and prevents false positives (e.g. ".well-known/jwks.json.bak")
        if req.path.ends_with("/.well-known/jwks.json") {
            res.set_header("Cache-Control", CACHE_CONTROL.to_string());
            res.set_header("X-Content-Type-Options", X_CONTENT_TYPE_OPTIONS.to_string());
            res.set_header("Vary", VARY.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify path-matching logic without constructing the full HandlerRequest.
    /// The middleware logic is: `req.path.ends_with("/.well-known/jwks.json")`.
    fn path_matches(path: &str) -> bool {
        path.ends_with("/.well-known/jwks.json")
    }

    #[test]
    fn test_matches_jwks_path() {
        assert!(path_matches("/.well-known/jwks.json"));
    }

    #[test]
    fn test_skips_non_jwks_path() {
        assert!(!path_matches("/api/v1/users"));
        assert!(!path_matches("/api/health"));
    }

    #[test]
    fn test_matches_prefix_paths() {
        assert!(path_matches("/v1/.well-known/jwks.json"));
        assert!(path_matches("/api/v1/.well-known/jwks.json"));
    }

    #[test]
    fn test_does_not_match_similar_paths() {
        assert!(!path_matches("/.well-known/jwks.json.bak"));
        assert!(!path_matches("/.well-known/jwks.json.extra"));
    }
}
