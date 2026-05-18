// User-owned controller for handler 'jwks'.

use crate::handlers::jwks::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(JwksController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "keys": [
    //     {
    //       "alg": "EdDSA",
    //       "crv": "Ed25519",
    //       "kid": "key-2026-05-18-12",
    //       "kty": "OKP",
    //       "use": "sig",
    //       "x": "pQUXMeHl6rK8cMDDGMhJvVfXw8SdJQ3lqRz5wLqNjKM"
    //     }
    //   ]
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "keys": [
    {
      "alg": "EdDSA",
      "crv": "Ed25519",
      "kid": "key-2026-05-18-12",
      "kty": "OKP",
      "use": "sig",
      "x": "pQUXMeHl6rK8cMDDGMhJvVfXw8SdJQ3lqRz5wLqNjKM"
    }
  ]
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {
        keys: vec![
            serde_json::json!({"alg":"EdDSA","crv":"Ed25519","kid":"key-2026-05-18-12","kty":"OKP","use":"sig","x":"pQUXMeHl6rK8cMDDGMhJvVfXw8SdJQ3lqRz5wLqNjKM"}),
        ],
    }
}
