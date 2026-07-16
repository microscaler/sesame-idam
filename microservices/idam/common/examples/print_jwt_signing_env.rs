//! Emit dotenv lines for `sesame-idam-jwt-signing` (Flux runtime secretGenerator).
//!
//! Usage (ms02):
//!   cd microservices/idam/common && cargo run --example print_jwt_signing_env \
//!     > ../../../../deployment-configuration/profiles/dev/sesame-idam/idam/runtime/jwt-signing.secrets.env
//!   sops --encrypt --in-place --input-type dotenv --output-type dotenv \
//!     deployment-configuration/profiles/dev/sesame-idam/idam/runtime/jwt-signing.secrets.env

use sesame_common::jwt::Ed25519Signer;

fn main() {
    let signer = Ed25519Signer::generate("dev-shared").expect("Ed25519 key generation failed");
    println!("kid={}", signer.kid());
    println!("pkcs8_b64={}", signer.pkcs8_b64());
}
