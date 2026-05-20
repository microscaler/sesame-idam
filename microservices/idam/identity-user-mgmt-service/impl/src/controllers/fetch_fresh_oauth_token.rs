
// Implementation stub for handler 'fetch_fresh_oauth_token'
// This file is a starting point for your implementation.
// You can modify this file freely - it will NOT be auto-regenerated.
// To regenerate this stub, use: brrtrouter-gen generate-stubs --path fetch_fresh_oauth_token --force

use brrtrouter_macros::handler;
use sesame_idam_identity_user_mgmt_service_gen::handlers::fetch_fresh_oauth_token::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;



/// Handler for Fetch Fresh Oauth Token.
/// Uses TTL configuration from `jwt::ttl::TtlConfig` to set `expires_in` on
/// issued access tokens.
#[handler(FetchFreshOauthTokenController)]
pub fn handle(req: TypedHandlerRequest<Request>) -> Response {
    // Apply TTL config for access and refresh token expiry.
    let ttl_config = crate::jwt::ttl::TtlConfig::from_env();
    let access_ttl_secs = ttl_config.access_ttl_secs_for_role("customer");
    let refresh_ttl_secs = ttl_config.refresh_ttl_for_role("customer").as_secs();
    ttl_config.record_ttl_metric("customer");

    Response {
        access_token: None, // TODO: Set from your business logic
        expires_in: Some(access_ttl_secs as i32),
        refresh_token: None, // TODO: Set from your business logic
        refresh_token_expires_in: Some(refresh_ttl_secs as i64),
        scope: None, // TODO: Set from your business logic
        token_type: None, // TODO: Set from your business logic
    }
}
