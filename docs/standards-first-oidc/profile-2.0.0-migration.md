# Provider profile 2.0.0: migration note

Date: 2026-10-11. Decision: [ADR-015](../ADR-015-signing-keys-and-hsm-custody.md).

## What changed

| | 1.0.0 | 2.0.0 |
| --- | --- | --- |
| ID token signature | EdDSA (Ed25519 keyset) | RS256 (RSA, key in Cloud KMS HSM in deployments) |
| Discovery `id_token_signing_alg_values_supported` | `["EdDSA"]` | `["RS256"]` (`["EdDSA"]` only in a development deployment without ID-token keys) |
| JWKS | OKP (Ed25519) keys | OKP keys and RSA keys (`kty`, `kid`, `use`, `alg`, `n`, `e`) |
| Access and refresh tokens | EdDSA, `typ=at+jwt` | unchanged |
| Claims, verified principal (`profile_version` 1.0.0) | | unchanged |

## OpenAPI diff

- identity-session-service `JWKS.keys[]`: `crv` and `x` are no longer required; `n` and `e` added; `kty` is `OKP` or
  `RSA`, `alg` is `EdDSA` or `RS256`.
- tenant-consumer: `x-provider-profile` 2.0.0, `x-fixture-version` 1.2.0; no operation or schema changes.

## Relying parties

- Validate ID tokens with RS256, selecting the key by `kid` from the JWKS (every OIDC library does by default).
- A relying party that pinned EdDSA for ID tokens must allow RS256 for them. Access-token validation stays EdDSA-only.
- A JWKS consumer that rejected unknown key types must now accept (or skip) RSA members.

## Fixtures

`conformance/oidc-v1`: `provider_profile` 2.0.0, `fixture_version` 1.2.0 (manifest gains `id_token_algorithm`),
checksum regenerated. `sesame-idam-client` supports 2.0.0 / 1.2.0.
