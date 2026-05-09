// User-owned controller for handler 'setup_user_mfa_totp'.

use crate::handlers::setup_user_mfa_totp::{Request, Response};
use brrtrouter::typed::TypedHandlerRequest;
use brrtrouter_macros::handler;

#[handler(SetupUserMfaTotpController)]
pub fn handle(_req: TypedHandlerRequest<Request>) -> Response {
    // Example response:
    // {
    //   "backup_codes": [
    //     "12345678",
    //     "87654321",
    //     "11111111",
    //     "22222222",
    //     "33333333"
    //   ],
    //   "mfa_required": true,
    //   "qr_code": "data:image/png;base64,example",
    //   "secret": "JBSWY3DPEHPK3PXP"
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "backup_codes": [
    "12345678",
    "87654321",
    "11111111",
    "22222222",
    "33333333"
  ],
  "mfa_required": true,
  "qr_code": "data:image/png;base64,example",
  "secret": "JBSWY3DPEHPK3PXP"
}"###,
    ) {
        Ok(parsed) => return parsed,
        Err(e) => {
            eprintln!("Failed to parse mock example JSON into Response: {}", e);
            // Fallback to empty default structs below
        }
    }

    Response {
        provisioning_uri: Some("example".to_string()),
        secret: Some("JBSWY3DPEHPK3PXP".to_string()),
        user_id: Some("example".to_string()),
    }
}
