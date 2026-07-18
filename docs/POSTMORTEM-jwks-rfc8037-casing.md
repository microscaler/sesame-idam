# Postmortem: JWKS emitted non-RFC-8037 casing, breaking token verification

- **Severity**: High (authentication broken for standards-compliant consumers)
- **Status**: Root-caused, fixed, fix committed (`8f4573c`); live rollout via pipeline
- **Owning repo**: sesame-idam (defect origin). Companion: BRRTRouter postmortem.
- **Author**: identity/platform

## Summary

`identity-session-service` published its JWKS with the wrong case for two
JOSE fields: `"kty":"okp"` and `"crv":"ED25519"`. RFC 8037 §2 mandates
**exactly** `"kty":"OKP"` and `"crv":"Ed25519"` (case-sensitive). Every
standards-compliant verifier that matches those literals rejected the key as
an unknown type/curve, could not find a usable key for the token's `kid`, and
returned `401` on **all** tokens. Because one malformed JWKS is shared by all
tenants and all relying parties, the failure looked like a broad
"multi-tenant token" outage rather than a single presentation bug.

The signing key itself was never wrong: it was correctly provisioned and
byte-identical across `identity-login-service` and `identity-session-service`
(sha256 match, `kid=dev-shared`). This was purely a JWKS *presentation*
defect on the public-key side.

## Impact

- **Confirmed broken**: opengroupware `og-auth` — a strict RFC-8037 verifier
  that skips any key whose `kty != "OKP"` or `crv != "Ed25519"`. It received
  zero usable keys and 401'd every request.
- **Tolerant (not broken by the casing)**: BRRTRouter's `JwksBearerProvider`
  (loadlinker/fleetingdns) matches `kty` case-insensitively and does not read
  `crv` at all — see the BRRTRouter postmortem. It was insulated from *this*
  specific defect, but only by leniency, and it has an adjacent latent trap
  (it required the OPTIONAL `alg` field) documented there.
- No data loss, no privilege escalation. Availability of authenticated
  endpoints only.

## Timeline (relative)

- **T0**: opengroupware began verifying sesame tokens against the live JWKS
  (`/idam/v1/.well-known/jwks.json`) and 401'd every valid token.
- **T1**: hypothesis of a per-tenant / hauliage-vs-loadlinker key mismatch.
- **T2**: cross-repo key-lifecycle trace. Per-tenant keys **disproven** —
  there is no per-tenant signing key anywhere; `issue_tokens()` takes
  `tenant_id` for claims only, the signer is tenant-independent.
- **T3**: live probe of the running JWKS revealed
  `{"kty":"okp","crv":"ED25519",…}` — the non-RFC casing.
- **T4**: root cause located in `key_manager.rs` serde attributes; fixed.

## Root cause

`identity-session-service/impl/src/key_manager.rs` serialized the JWK type
and curve enums with `#[serde(rename_all = …)]` that produced the wrong case:

```rust
#[serde(rename_all = "lowercase")]              // Okp     -> "okp"
pub enum JwkKeyType { Okp }
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]   // Ed25519 -> "ED25519"
pub enum JwkCurve { Ed25519 }
```

The `Display` impls returned the correct `"OKP"`/`"Ed25519"`, which made the
code *look* right on inspection — but serde serialization ignores `Display`
and used the `rename_all` casing on the wire. The JWKS handler serializes
these enums directly into the response, so the public endpoint emitted the
wrong case.

**Fix** (`8f4573c`): drop the `rename_all` attributes; put an explicit
`#[serde(rename = "OKP")]` on `Okp`; `JwkCurve::Ed25519` serializes as its
variant name `"Ed25519"`. The JWKS now emits RFC-8037 casing.

## Why it wasn't caught earlier

1. **A test enshrined the bug.** `tests/bdd/jwks_http.rs` asserted
   `kty == "okp"` and `crv == "ED25519"` — it validated the *wrong* output,
   so it passed while the defect shipped. (Now corrected to assert the RFC
   casing.)
2. **No consumer contract test.** Nothing in CI fetched the JWKS and verified
   a real token against it with a standards-compliant library. Unit tests
   round-tripped through sesame's own lenient `String`-typed cache, which
   never enforced casing.
3. **`Display` masked the defect in review.** The correct-looking `Display`
   impls diverted attention from the serde attributes actually used on the
   wire.

## Corrective actions

| # | Action | Owner | Status |
|---|--------|-------|--------|
| 1 | Emit RFC-8037 casing (`OKP`/`Ed25519`) | sesame | Done (`8f4573c`) |
| 2 | Fix the BDD test to assert RFC casing | sesame | Done (`8f4573c`) |
| 3 | Roll the fix to the live cluster | sesame | In progress (pipeline) |
| 4 | Add a JWKS *contract* test that signs a token and verifies it with a strict RFC-8037 verifier (see opengroupware `og-auth::mock::MockIdp`) and run it in CI | sesame | TODO |
| 5 | Consider serializing JWKs via a hand-written, RFC-cited struct (string literals, no `rename_all`) to remove the `Display`-vs-serde divergence class entirely | sesame | TODO |
| 6 | BRRTRouter: accept OPTIONAL `alg`, log skipped keys loudly | BRRTRouter | Done (see companion) |

## Prevention principle

Any service that publishes a JOSE/JWKS document must have a CI test that a
**third-party, spec-strict** verifier accepts its output — not just its own
parser. Self-consistency (sign here, verify here) hides presentation bugs
because the same lenient code sits on both ends. The
`og-auth --features mock` `MockIdp` round-trip test is the reference pattern;
mirror it for every JOSE producer.
