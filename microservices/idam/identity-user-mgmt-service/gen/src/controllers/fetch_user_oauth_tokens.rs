// User-owned controller for handler 'fetch_user_oauth_tokens'.

use crate::handlers::fetch_user_oauth_tokens::{Request, Response};
use brrtrouter::typed::HttpJson;
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(FetchUserOauthTokensController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> HttpJson<Response> {
    // Example response:
    // {
    //   "oauth_tokens": [
    //     {
    //       "created_at": "2024-01-10T00:00:00Z",
    //       "expires_at": null,
    //       "id": "550e8400-e29b-41d4-a716-446655440001",
    //       "provider": "github",
    //       "provider_user_id": "12345",
    //       "scope": "repo,user"
    //     },
    //     {
    //       "created_at": "2024-01-12T00:00:00Z",
    //       "expires_at": "2024-07-12T00:00:00Z",
    //       "id": "550e8400-e29b-41d4-a716-446655440002",
    //       "provider": "google",
    //       "provider_user_id": "67890",
    //       "scope": "email,profile"
    //     }
    //   ]
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "oauth_tokens": [
    {
      "created_at": "2024-01-10T00:00:00Z",
      "expires_at": null,
      "id": "550e8400-e29b-41d4-a716-446655440001",
      "provider": "github",
      "provider_user_id": "12345",
      "scope": "repo,user"
    },
    {
      "created_at": "2024-01-12T00:00:00Z",
      "expires_at": "2024-07-12T00:00:00Z",
      "id": "550e8400-e29b-41d4-a716-446655440002",
      "provider": "google",
      "provider_user_id": "67890",
      "scope": "email,profile"
    }
  ]
}"###,
    ) {
        Ok(parsed) => return HttpJson::ok(parsed),
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    HttpJson::ok(Response {})
}
