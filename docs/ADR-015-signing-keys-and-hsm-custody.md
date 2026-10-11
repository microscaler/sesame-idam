# ADR-015: Signing keys by purpose, and which live in an HSM

> **Status:** ACCEPTED (2026-10-11)
> **Related:** [ADR-006](ADR-006-shared-signing-keys-for-ha.md) (shared Ed25519 keyset),
> [ADR-007](ADR-007-tenant-domain-verification.md) amendment of 2026-10-11 (`K_enroll` in an HSM),
> [ADR-014](ADR-014-saml-signature-verification.md) (SAML), [security profile](standards-first-oidc/security-profile-v1.md)

---

## Context

Until now Sesame signed everything with one Ed25519 keyset (ADR-006): access tokens, refresh tokens and ID tokens.
Three things change that:

1. **OpenID Connect requires RS256 for ID tokens.** OpenID Connect Core §15.1 makes RS256 mandatory to implement for
   ID tokens; certification (Epic 14) and many relying-party libraries expect it, and not all support EdDSA.
2. **SAML needs RSA.** The AuthnRequest signature (ADR-014 §5) must be RSA-SHA256: Entra ID accepts only RSA-SHA256
   for signed authentication requests.
3. **Cloud KMS can keep keys in an HSM**, but not every algorithm: HMAC, RSA and ECDSA are offered at HSM protection
   level; Ed25519 only in software. Every KMS signature is a network call (tens of milliseconds) and a dependency.

Verification never needs the private key: services verify with public keys from the JWKS, cached locally. Only
signing touches custody.

## Decision

### 1. One key per purpose

| Purpose | Algorithm | Custody (GCP deployments) | On the hot path? |
|---|---|---|---|
| Access and refresh tokens | Ed25519 (`EdDSA`) | software keyset, ADR-006 (unchanged) | yes: every refresh |
| ID tokens | RS256 (RSA-2048, PKCS#1 v1.5, SHA-256) | Cloud KMS, HSM (`RSA_SIGN_PKCS1_2048_SHA256`) | no: issued at the code exchange |
| SAML AuthnRequest | RSA-SHA256 (RSA-3072) | Cloud KMS, HSM (`RSA_SIGN_PKCS1_3072_SHA256`) | no: once per SAML sign-in |
| Domain-verification tokens (`K_enroll`) | HMAC-SHA256 | Cloud KMS, HSM (ADR-007 amendment) | no: daily checks |

Access tokens stay Ed25519 in software, so token refreshes, the most frequent background operation, never wait on
KMS. The keys that sign at interactive moments (the code exchange, the start of a SAML sign-in) or in background jobs
live in the HSM from day one; their latency is hidden behind user steps or irrelevant.

### 2. The key decides the algorithm, never the token

The JWKS publishes Ed25519 (`OKP`) and RSA keys side by side, each with `alg` set. A verifier looks up the key by
`kid` and uses that key's `alg`; the token header's `alg` must equal it. Each verifier also pins what it accepts for
its purpose: Sesame's access-token validation accepts only `EdDSA` with `typ=at+jwt` (an RS256 ID token can never pass
as an access token); relying parties validating ID tokens accept `RS256`.

### 3. Signers behind one interface, KMS or local

`sesame_common::jwt::rs256` signs RS256 JWTs through a signer with two backends, configured by the JSON file named in
`SESAME_ID_TOKEN_KEYS` (Cloud KMS key versions or PEM files, current first):

- **Cloud KMS** (`asymmetricSign` over the SHA-256 digest): Workload Identity, request and response CRC32C, the
  answering key version and protection level `HSM` checked, as for `K_enroll`. The public key comes from
  `getPublicKey`; the JWKS publisher needs only that (`roles/cloudkms.publicKeyViewer`), never the signing permission.
- **Local PKCS#8 key file** (development and tests): signed with ring.

The `kid` of an RSA key is its RFC 7638 thumbprint. Rotation: a new KMS key version becomes current; the previous one
stays published in the JWKS until every ID token it signed has expired.

### 4. Metadata stays honest

Discovery's `id_token_signing_alg_values_supported` lists `RS256` when an RS256 signer is configured, and `EdDSA`
only for a deployment without one (development fallback). The ID-token algorithm is a locked rule of the security
profile, so this is `provider_profile` 2.0.0, done in lockstep (`compatibility-v1.md`: profile documents, fixture
`VERSION` and `CHECKSUM`, the tenant-consumer OpenAPI, `sesame-idam-client`'s supported version) before any deployment
sets `SESAME_ID_TOKEN_KEYS`. Done 2026-10-11: [`profile-2.0.0-migration.md`](standards-first-oidc/profile-2.0.0-migration.md).

### 5. HSM-backed access tokens are not adopted now

Moving access tokens into an HSM would mean ES256 or RS256 (no Ed25519 HSM), a KMS call on every refresh and a format
change for every verifier. Not justified while the keyset is born in the secret backend and rotated (ADR-006); a new
ADR if that changes.

## Consequences

- The JWKS carries two key types; Sesame's JWKS schema allows RSA members (`n`, `e`) next to OKP ones (`crv`, `x`).
  Sesame's verifiers (sesame-common, BRRTRouter's JWKS provider) already parse both and select by `kid`.
- ID tokens grow by about 300 bytes (a 256-byte RSA signature instead of 64).
- The code exchange depends on KMS: if KMS is unreachable, `/oauth/token` fails for authorization-code clients
  (refreshes do not). Cost: about $1 a month per key version and $0.15 per 10,000 signatures (October 2026 list prices).
- Deployments add two KMS keys (ID token, SAML SP) next to `K_enroll`, with `roles/cloudkms.signer` for
  identity-login-service and `roles/cloudkms.publicKeyViewer` for identity-session-service.

## Alternatives rejected

- **Everything Ed25519**: fails the OIDC mandatory-to-implement algorithm and SAML with Entra ID.
- **Everything in the HSM**: puts KMS on the refresh path for no gain in a threat the keyset rotation already bounds.
- **ES256 for SAML**: Entra ID does not accept it.
