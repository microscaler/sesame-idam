# OIDC conformance package v1

Language-neutral fixtures for Sesame provider profile `1.0.0`.

## Versions

See [`VERSION`](./VERSION):

- `provider_profile` — normative profile semver
- `fixture_version` — corpus revision

Recorded checksum: [`CHECKSUM`](./CHECKSUM) (SHA-256 over canonical
`manifest.json` + `protocol-cases.json`).

## Load instructions

No TypeScript outside `/ui`. Prefer Python or Rust:

```bash
# Validate corpus + print checksum
cd tooling && source ../.venv/bin/activate  # or: just init
python -m sesame_idam_tooling.oidc_conformance

# Contract lockstep (profile ↔ OpenAPI ↔ schema ↔ VERSION)
python -m sesame_idam_tooling.contract_sync

# Authlib proof consumer (Epic 15.8)
pip install 'Authlib==1.3.2'
python -m sesame_idam_tooling.authlib_contract_proof
```

Rust clients should vendor or path-reference this directory and assert
`provider_profile` / `fixture_version` match their supported matrix.

## Contents

| File | Purpose |
|---|---|
| `manifest.json` | Case index + redaction list |
| `protocol-cases.json` | Accept/reject fixtures by family |
| `VERSION` | Profile + fixture versions |
| `CHECKSUM` | Release gate digest |

## CI

Epic 14 job `oidc-conformance-gate` remains the protocol regression gate.
Epic 15 adds `contract_sync` in the same workflow.
