//! Generate a fresh Ed25519 JWT signing key and print it as a Kubernetes
//! Secret manifest (`sesame-idam-jwt-signing`) consumed by the Helm chart.
//!
//! Usage (see `just jwt-signing-secret`):
//!
//! ```text
//! cargo run -q -p sesame-common --bin sesame_keygen | kubectl apply -f -
//! ```
//!
//! identity-login-service signs access tokens with this key;
//! identity-session-service publishes its public half at
//! `/.well-known/jwks.json`. Rotate by re-running and restarting both
//! services (session-service serves old + new during its grace window).

use sesame_common::jwt::Ed25519Signer;

fn main() {
    let now = chrono::Utc::now();
    let kid = format!("key-{}", now.format("%Y-%m-%d-%H%M"));

    let signer = Ed25519Signer::generate(&kid).expect("Ed25519 key generation failed");

    println!(
        "\
apiVersion: v1
kind: Secret
metadata:
  name: sesame-idam-jwt-signing
  namespace: sesame-idam
  labels:
    app.kubernetes.io/part-of: sesame-idam
type: Opaque
stringData:
  kid: \"{kid}\"
  pkcs8_b64: \"{pkcs8}\"
",
        kid = signer.kid(),
        pkcs8 = signer.pkcs8_b64(),
    );

    eprintln!(
        "generated Ed25519 signing key kid={} (public x={})",
        signer.kid(),
        signer.public_jwk_x().expect("public key")
    );
}
