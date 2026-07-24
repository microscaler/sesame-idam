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

use sesame_common::jwt::{rfc7638_okp_thumbprint, Ed25519Signer};

/// `sesame_keygen keyset [n]` — emit a Kubernetes Secret carrying an
/// ADR-006 shared signing KEYSET (`sesame-idam-signing-keyset` /
/// `signing-keyset.json`) with `n` keys (default 1; extras are backdated to
/// act as grace keys). Pipe into SOPS for the deployment-configuration
/// profile (step 1) — step 2 moves generation into the secret backend.
fn keyset_main(n: usize) {
    let now = chrono::Utc::now();
    let mut entries = Vec::new();
    let mut kids = Vec::new();
    for i in 0..n.max(1) {
        let signer = Ed25519Signer::generate("keyset").expect("Ed25519 key generation failed");
        let x = signer.public_jwk_x().expect("public key");
        let kid = rfc7638_okp_thumbprint(&x);
        // First key active now; subsequent keys backdated 30d apart (grace).
        let valid_from = now - chrono::Duration::days(30 * i as i64);
        entries.push(serde_json::json!({
            "pkcs8_b64": signer.pkcs8_b64(),
            "valid_from": valid_from.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        }));
        kids.push(kid);
    }
    let keyset = serde_json::json!({ "keys": entries });
    let keyset_json =
        serde_json::to_string_pretty(&keyset).expect("keyset serialization failed");

    println!(
        "\
apiVersion: v1
kind: Secret
metadata:
  name: sesame-idam-signing-keyset
  namespace: sesame-idam
  labels:
    app.kubernetes.io/part-of: sesame-idam
type: Opaque
stringData:
  signing-keyset.json: |
{keyset_indented}
",
        keyset_indented = keyset_json
            .lines()
            .map(|l| format!("    {l}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    eprintln!("generated {} key(s), kids (RFC 7638): {}", n.max(1), kids.join(", "));
    eprintln!("SOPS-encrypt this Secret into deployment-configuration; mount via helm signingKeyset.enabled=true");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("keyset") {
        let n = args
            .get(2)
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(1);
        keyset_main(n);
        return;
    }

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
