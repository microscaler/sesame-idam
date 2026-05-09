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
    //       "alg": "RS256",
    //       "e": "AQAB",
    //       "kid": "default",
    //       "kty": "RSA",
    //       "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbApbghMi8nF-_S0AI4-eJad0a30iV3-V2XN0b4g9S1_Hk09HM5y1nVAGTovsJ34vcEe",
    //       "use": "sig"
    //     }
    //   ]
    // }
    match serde_json::from_str::<Response>(
        r###"{
  "keys": [
    {
      "alg": "RS256",
      "e": "AQAB",
      "kid": "default",
      "kty": "RSA",
      "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbApbghMi8nF-_S0AI4-eJad0a30iV3-V2XN0b4g9S1_Hk09HM5y1nVAGTovsJ34vcEe",
      "use": "sig"
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
            serde_json::json!({"alg":"RS256","e":"AQAB","kid":"default","kty":"RSA","n":"0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbApbghMi8nF-_S0AI4-eJad0a30iV3-V2XN0b4g9S1_Hk09HM5y1nVAGTovsJ34vcEe","use":"sig"}),
        ],
    }
}
