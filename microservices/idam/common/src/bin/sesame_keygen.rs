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

/// `sesame_keygen keyset [n] [--out PATH] [--sops]` — emit a Kubernetes
/// Secret carrying an ADR-006 shared signing KEYSET
/// (`sesame-idam-signing-keyset` / `signing-keyset.json`) with `n` keys
/// (default 1; extras are backdated to act as grace keys).
///
/// - Without `--out`: manifest on stdout (dev inspection).
/// - `--out PATH`: write the manifest to PATH — use the SOPS-ruled location
///   `deployment-configuration/profiles/<env>/.../signing-keyset.secret.yaml`
///   (the repo's `.sops.yaml` `*.secret.yaml` rule encrypts only
///   data/stringData). Private material never hits stdout in this mode.
/// - `--sops`: after writing, run `sops -e -i PATH` so the plaintext file
///   never outlives the command. If sops fails, the plaintext file is
///   DELETED and the command errors — regenerate rather than risk plaintext
///   key material in git.
///
/// Step 2 (ADR-006) moves generation into the secret backend entirely.
fn keyset_main(n: usize, out: Option<&str>, encrypt: bool) {
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

    let manifest = format!(
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

    eprintln!(
        "generated {} key(s), kids (RFC 7638): {}",
        n.max(1),
        kids.join(", ")
    );

    let Some(path) = out else {
        if encrypt {
            eprintln!("error: --sops requires --out PATH (sops needs the file path for its creation rules)");
            std::process::exit(2);
        }
        println!("{manifest}");
        eprintln!("tip: use --out deployment-configuration/profiles/<env>/.../signing-keyset.secret.yaml --sops");
        return;
    };

    if let Err(e) = std::fs::write(path, &manifest) {
        eprintln!("error: writing {path}: {e}");
        std::process::exit(1);
    }
    eprintln!("wrote {path} (PLAINTEXT until encrypted)");

    if encrypt {
        // Encrypt in place at the FINAL path so .sops.yaml creation rules
        // (path_regex on *.secret.yaml) apply. On failure, delete the
        // plaintext — keys are cheap, plaintext key material in git is not.
        match std::process::Command::new("sops").args(["-e", "-i", path]).status() {
            Ok(status) if status.success() => {
                eprintln!("encrypted in place with sops: {path}");
            }
            other => {
                let _ = std::fs::remove_file(path);
                eprintln!(
                    "error: sops -e -i {path} failed ({other:?}) — plaintext file DELETED. \
                     Check sops/.sops.yaml (the path must match a creation rule, e.g. \
                     deployment-configuration/profiles/**/*.secret.yaml) and re-run."
                );
                std::process::exit(1);
            }
        }
    } else {
        eprintln!("next: sops -e -i {path}");
    }
    eprintln!("then: helm signingKeyset.enabled=true on identity-login + identity-session");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("keyset") {
        let mut n: usize = 1;
        let mut out: Option<String> = None;
        let mut encrypt = false;
        let mut iter = args.iter().skip(2);
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--out" | "-o" => match iter.next() {
                    Some(p) => out = Some(p.clone()),
                    None => {
                        eprintln!("error: --out requires a path");
                        std::process::exit(2);
                    }
                },
                "--sops" => encrypt = true,
                v => match v.parse::<usize>() {
                    Ok(parsed) => n = parsed,
                    Err(_) => {
                        eprintln!("error: unrecognised argument {v}");
                        eprintln!("usage: sesame_keygen keyset [n] [--out PATH] [--sops]");
                        std::process::exit(2);
                    }
                },
            }
        }
        keyset_main(n, out.as_deref(), encrypt);
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
