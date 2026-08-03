# Release conformance gate (Epic 14.9)

Protocol regressions are **release-blocking**. Provider profile bumps require
external suite evidence.

## Named CI job

**Job:** `oidc-conformance-gate` in [`.github/workflows/ci.yaml`](../../../../.github/workflows/ci.yaml)

Steps:

1. Validate `conformance/oidc-v1` corpus + print `fixture_checksum`
2. Validate framework-matrix fixture binding
3. Run Rust filter: `oidc_conformance` (+ unit tests for `verify_access_token` / redaction)

## Local commands

```bash
# Fixture checksum + corpus validation (ms02)
cd ~/Workspace/microscaler/sesame-idam
source .venv/bin/activate
python -m sesame_idam_tooling.oidc_conformance
python -m sesame_idam_tooling.oidc_framework_matrix

# Protocol BDD
cd microservices
cargo nextest run -p sesame_idam_identity_login_service oidc_conformance
cargo nextest run -p sesame-common verified_access redaction
```

## Policy

| Change | Required evidence |
|--------|-------------------|
| Any OIDC protocol / validator change | Green `oidc-conformance-gate` |
| `provider_profile` semver bump | OIDF Basic/PKCE report under `evidence/` + matrix note |
| Fixture corpus change | Checksum changes intentionally; update evidence if consumer-visible |
