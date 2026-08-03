# OIDC / Epic resume — 2026-08-03

## Epic 14

- Landed on `main` (tip lineage includes `feat(oidc): Epic 14 conformance gate and product-neutral fixtures`).
- Product-neutral fixtures: `acme` / `globex` (no Hauliage/PriceWhisperer in OSS contract).

## Epic 15 — Portable Consumer Contract (COMPLETE lineup)

Status: **In progress** in INDEX/README; all plan waves implemented.

| Wave | Status |
|---|---|
| 0 rebase/push Epic 14 | Done (`origin/main` current) |
| 1 Normative freeze | Done — provider-profile entry + claim tables + verified-principal mapping/tests |
| 2 Public API | Done — tenant-consumer OpenAPI + transport-policy + live contract BDD |
| 3 Boundaries/fixtures | Done — client-boundaries + VERSION/CHECKSUM + contract_sync |
| 4 Proof | Done — quickstarts; sesame-idam-client lockstep; Authlib proof |
| 5 Compatibility | Done — compatibility-v1.md + acceptance evidence |

### Key paths

- `docs/standards-first-oidc/provider-profile-v1.md`
- `docs/standards-first-oidc/verified-principal-mapping-v1.md`
- `docs/standards-first-oidc/transport-policy-v1.md`
- `docs/standards-first-oidc/client-boundaries-v1.md`
- `docs/standards-first-oidc/compatibility-v1.md`
- `docs/standards-first-oidc/quickstarts/`
- `openapi/idam/tenant-consumer/openapi.yaml`
- `conformance/oidc-v1/{VERSION,CHECKSUM,README.md}`
- `microservices/idam/common/src/jwt/verified_principal.rs`
- `tooling/src/sesame_idam_tooling/{contract_sync,authlib_contract_proof,verified_principal}.py`
- Sibling: `sesame-idam-client` optional `org_id` + tenant-consumer contract_sync

### Verify

```bash
ssh ms02 'cd ~/Workspace/microscaler/sesame-idam && PYTHONPATH=tooling/src python3 -m sesame_idam_tooling.contract_sync'
ssh ms02 'source ~/.cargo/env && cd ~/Workspace/microscaler/sesame-idam/microservices && cargo nextest run -p sesame-common verified_principal'
ssh ms02 'source ~/.cargo/env && cd ~/Workspace/microscaler/sesame-idam/microservices && cargo nextest run -p sesame_idam_identity_login_service tenant_consumer_live_contract'
```

### Not committed yet

Local working tree changes for Epic 15 (and client repo) — commit when user asks.
