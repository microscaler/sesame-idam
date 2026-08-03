# Runbook: Generate JWT signing keys for SOPS (JWKS)

Status: publishable  
Audience: platform operators rotating or bootstrapping Sesame signing material  
Tools: `sesame_keygen` (`sesame-common`), `just keyset-secret`, `just jwt-signing-material`, SOPS  
Related: [ADR-006](../ADR-006-shared-signing-keys-for-ha.md), [SOPS age keys](../sops-age-keys.md)

Sesame access tokens are **Ed25519** (`alg=EdDSA`, `typ=at+jwt`). Login and
session services must share the same private material so JWKS `kid` values match
minted tokens. That material is generated with an in-repo tool, written into a
GitOps path under `deployment-configuration/`, and **SOPS-encrypted** before
commit.

## Which artifact to generate

| Artifact | Tool | Git path (dev example) | Cluster Secret | When |
|---|---|---|---|---|
| **Signing keyset** (preferred, ADR-006) | `sesame_keygen keyset` / `just keyset-secret` | `…/runtime/signing-keyset.secret.yaml` | `sesame-idam-signing-keyset` | Multi-replica / HA; `KEY_SOURCE=file` |
| **Single-key dotenv** (legacy Flux generator) | `print_jwt_signing_env` / `just jwt-signing-material` | `…/runtime/jwt-signing.secrets.env` | `sesame-idam-jwt-signing` | Older env-pair (`kid` + `pkcs8_b64`) still referenced by some profiles |

Prefer the **keyset** path for new environments. Keep the dotenv file only where
the profile’s `kustomization.yaml` still lists it.

## Prerequisites

- Run generation on a trusted machine with Rust toolchain (typically **ms02**).
- SOPS installed; age private identity discoverable (see
  [sops-age-keys.md](../sops-age-keys.md)).
- Repo `.sops.yaml` creation rules cover:
  - `deployment-configuration/profiles/**/*.secret.yaml` (encrypts `data` /
    `stringData` only)
  - `deployment-configuration/profiles/**/*.secrets.env` (whole dotenv)

Encrypting needs only the **public** recipients in `.sops.yaml`. Decrypting /
editing needs your private age key.

## Preferred: shared signing keyset (ADR-006)

### What you get

A Kubernetes Secret manifest whose `stringData.signing-keyset.json` is a JSON
document of one or more Ed25519 keys (`pkcs8_b64` + `valid_from`). Kids are
**RFC 7638** thumbprints of the public keys. Session publishes all public halves
at JWKS; login signs with the newest key whose `valid_from` has passed.

### Generate and encrypt in one step

From the Sesame repository root on ms02:

```bash
# n = number of keys (1 = current only; 2 = current + grace)
just keyset-secret 2 \
  deployment-configuration/profiles/dev/sesame-idam/idam/runtime/signing-keyset.secret.yaml
```

Equivalent without `just`:

```bash
cd microservices
cargo run -q -p sesame-common --bin sesame_keygen keyset 2 \
  --out ../deployment-configuration/profiles/dev/sesame-idam/idam/runtime/signing-keyset.secret.yaml \
  --sops
```

Behavior:

1. Generates `n` Ed25519 keys.
2. Writes `signing-keyset.secret.yaml` (Secret name `sesame-idam-signing-keyset`).
3. Runs `sops -e -i <path>` so the file matches `.sops.yaml` rules.
4. If SOPS fails, the **plaintext file is deleted** — regenerate rather than
   risk committing private keys.

Inspect without writing:

```bash
just keyset-secret 1
# or: cargo run -q -p sesame-common --bin sesame_keygen keyset 1
```

### After encrypting

1. Confirm ciphertext: `stringData` / `signing-keyset.json` show `ENC[AES256_GCM,…]`
   and a `sops:` metadata block — never readable PKCS#8.
2. Commit the encrypted YAML.
3. Let GitOps reconcile the `sesame-idam` runtime kustomization (it must include
   `signing-keyset.secret.yaml`).
4. Ensure login + session have `signingKeyset.enabled` (or equivalent) so they
   mount the Secret and set `KEY_SOURCE=file` + `SESAME_SIGNING_KEYSET_FILE`.
5. Roll identity-login-service and identity-session-service.
6. Verify:

   ```bash
   # JWKS must list the new kid(s)
   curl -sS https://id.<zone>/.well-known/jwks.json | jq .

   # Fresh login token header kid must be in that JWKS
   ```

### Rotation (keyset)

1. Generate a new keyset with `n≥2` (or append via your rotation process) using
   `just keyset-secret … --sops` to the same path.
2. Commit, reconcile, roll pods.
3. Keep the previous key in the keyset until outstanding access tokens expire
   (grace). Dropping the old key early causes intermittent `invalid_token`.

Do not leave a plaintext `signing-keyset.secret.yaml` on shared NFS.

## Legacy: single-key dotenv (`jwt-signing.secrets.env`)

Still used where Flux `secretGenerator` builds `sesame-idam-jwt-signing` from a
dotenv file (`kid=` / `pkcs8_b64=`).

```bash
# From repo root on ms02
just jwt-signing-material \
  > deployment-configuration/profiles/dev/sesame-idam/idam/runtime/jwt-signing.secrets.env

# Encrypt immediately (dotenv input/output types)
sops --encrypt --in-place --input-type dotenv --output-type dotenv \
  deployment-configuration/profiles/dev/sesame-idam/idam/runtime/jwt-signing.secrets.env
```

Underlying example binary:

```bash
cd microservices/idam/common
cargo run --example print_jwt_signing_env
```

Then commit the encrypted dotenv, reconcile, restart login + session. Token
`kid` must match JWKS.

**Direct kubectl apply** (ephemeral / emergency only — not GitOps):

```bash
just jwt-signing-secret
# cargo run -q -p sesame-common --bin sesame_keygen | kubectl apply -f -
```

That prints a one-shot Secret to the cluster without updating the SOPS file.
Prefer regenerating the GitOps artifact above for durable environments.

## Profile path template

```text
deployment-configuration/profiles/<profile>/sesame-idam/idam/runtime/
  signing-keyset.secret.yaml   # ADR-006 keyset (preferred)
  jwt-signing.secrets.env      # legacy single key (if still listed in kustomization)
```

Replace `<profile>` (`dev`, staging, …) to match the environment you are
bootstrapping.

## SOPS tips

- Age discovery and the macOS path trap:
  [sops-age-keys.md](../sops-age-keys.md).
- After changing `.sops.yaml` recipients: `sops updatekeys <file>` then commit.
- Never copy a plaintext example into place and leave it unencrypted on a shared
  host — encrypt in the same command session (`--sops` or immediate `sops -e -i`).

## Failure modes

| Symptom | Likely cause |
|---|---|
| Login works, APIs `401 invalid_token` | Login and session using different keys / kids |
| JWKS empty or wrong kid | Session not loading keyset file; Secret not mounted |
| `sops -e` fails; file deleted | Path outside `.sops.yaml` rules, or sops missing |
| Decrypt fails on Mac | Age key in `~/.config` instead of Library path |
| Multi-replica JWKS disagreement | Still on ephemeral in-memory keys; enable keyset mode |

## Security checklist

- [ ] Generated only on a trusted host
- [ ] Encrypted before `git add` / never committed as plaintext
- [ ] Login and session consume the **same** Secret
- [ ] Rotation retains a grace key until tokens expire
- [ ] Age admin key not stored in the cluster; Flux recipient decrypts in-cluster
