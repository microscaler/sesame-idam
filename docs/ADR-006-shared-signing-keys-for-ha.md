# ADR-006: Shared Ed25519 signing keys for identity-session-service HA

Status: PROPOSED (2026-07-22)
Context links: `docs/POSTMORTEM-2026-07-22-jwks-casing-login-outage.md`,
`microservices/idam/identity-session-service/impl/src/key_manager.rs`

## Context

`identity-session-service` signs access tokens with Ed25519 keys held **in
process memory only** (see key_manager.rs module contract): never serialized,
fresh keypair on restart, self-rotation with a grace-period overlap. This has
two structural consequences:

1. **Restart invalidates every outstanding session** (new keypair, new kid).
2. **Multi-replica is unsafe**: each pod would generate its own keypair;
   JWKS requests load-balance across disagreeing key sets; verification fails
   intermittently and undebuggably.

Scaling past one replica is a certainty, not a possibility. The platform
already has the delivery shape for shared secrets:
`deployment-configuration` (SOPS-encrypted app secrets in git) →
`secret-manager-controller` pushes to a backend (OpenBao or GCP Secret
Manager) → `external-secrets` materializes a cluster `Secret` → pods read it
(mounted, not baked into images).

## Decision

Adopt **file-sourced shared signing keysets** delivered through the secrets
chain, with one deliberate refinement over the standard app-secret flow:

### 1. The private key is BORN in the secret backend — never in git

`application.secrets.env` / SOPS-git is the right channel for passwords; it is
the wrong channel for token-signing keys: a signing key in git (even
encrypted) means any past holder of the age key can mint valid tokens forever,
and the key's history is retained indefinitely. Instead:

- **OpenBao backend**: the key is generated inside OpenBao.
- **GCP SM backend**: `secret-manager-controller` generates the keypair and
  writes it, with **generate-if-absent** semantics (this is a new controller
  feature: a `generate: ed25519` field on the secret spec — filed against
  `gcp-secret-manager-controller`).
- **Git carries only the reference**: an `ExternalSecret` manifest naming the
  backend path. Git describes *that* the key exists and *where* — never *what
  it is*.

### 2. The Secret carries a KEYSET, not a key

JSON document, ordered newest-first, two entries (current + previous):

```json
{ "keys": [
    { "kid": "<rfc7638-thumbprint>", "d": "<b64url private>", "x": "<b64url public>", "valid_from": "..." },
    { "kid": "...", "d": "...", "x": "...", "valid_from": "..." }
] }
```

- Every replica publishes **all** public keys in JWKS and signs with the
  newest entry whose `valid_from` has passed — this preserves the existing
  in-memory design's rotation-overlap property across replicas AND restarts.
- Rotation = controller appends a new entry and drops the oldest after the
  grace period (a secret-version write; external-secrets propagates it).
- Sessions now SURVIVE pod restarts — repairing consequence (1) above as a
  side effect.

### 3. Deterministic kids (RFC 7638)

`kid` = JWK thumbprint of the public key, replacing timestamp kids. N replicas
agree on every kid with zero coordination; the same key always has the same
name everywhere.

### 4. Reload without restart

external-secrets updates the cluster Secret; kubelet refreshes the mounted
file (~1 min); `KeyManager` watches the file and reloads the keyset. Pod
rolls are unnecessary precisely because of the overlap window — the old key
verifies while the new one phases in.

### 5. Dev mode unchanged

`KEY_SOURCE=ephemeral|file` (default `ephemeral`): Tilt/kind development keeps
today's zero-dependency in-memory behavior; shared-k8s and production set
`file` with the mounted keyset. One branch in `KeyManager::bootstrap`.

## Consequences

- (+) Safe horizontal scaling of identity-session-service.
- (+) Sessions survive restarts and rotations.
- (+) Key custody moves to purpose-built stores (OpenBao/GCP SM) with their
  audit, IAM and versioning — instead of process memory OR git.
- (−) The "private keys never leave the process" property is deliberately
  traded for HA. Mitigations: backend-side generation (never in git), backend
  IAM + audit logging, kid thumbprints make any unexpected signer visible in
  JWKS immediately.
- (−) New dependency for shared/prod: the secrets chain must be healthy for
  *bootstrap* (running pods keep their loaded keyset if the chain degrades).
- (−) Requires the `generate: ed25519` / generate-if-absent feature in
  `secret-manager-controller` before this ADR can be implemented for the GCP
  path; OpenBao path can proceed first.

## Implementation order

1. `KeyManager`: `KEY_SOURCE=file` branch — load/watch/reload keyset file;
   RFC 7638 kids; sign-with-newest-valid; publish-all. (`ephemeral` path
   untouched.)
2. deployment-configuration: `ExternalSecret` + volume mount for
   identity-session-service in the shared profile.
3. OpenBao path end-to-end (backend-side generation).
4. `gcp-secret-manager-controller`: `generate: ed25519` feature; then GCP
   path.
5. Rotation job (controller cron or Bao policy): append-new / expire-oldest on
   the existing 30d cadence.
6. Flip shared-k8s profile to `KEY_SOURCE=file`; scale to 2 replicas; verify
   JWKS agreement and login continuity through a forced rotation.
