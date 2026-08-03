# Epic 15 wave completion evidence — 2026-08-03

## Waves

| Wave | Result |
|---|---|
| 0 Ship Epic 14 | `main` tip includes Epic 14 conformance gate (`9b23d2b` lineage) |
| 1 Normative freeze | Provider profile TOC + claim tables; verified-principal mapping; schema tests |
| 2 Public API | Hardened tenant-consumer OpenAPI; transport-policy-v1; live contract BDD |
| 3 Boundaries + fixtures | client-boundaries-v1; VERSION/CHECKSUM/README package; contract_sync |
| 4 Proof consumers | Quickstarts; sesame-idam-client lockstep; Authlib proof module |
| 5 Versioning | compatibility-v1.md; Epic acceptance checkboxes evidenced |

## Commands

```bash
python -m sesame_idam_tooling.oidc_conformance
python -m sesame_idam_tooling.contract_sync
pip install 'Authlib==1.3.2'
python -m sesame_idam_tooling.authlib_contract_proof
cd microservices && cargo nextest run -p sesame-common verified_principal
cd microservices && cargo nextest run -p sesame_idam_identity_login_service tenant_consumer_live_contract
```
