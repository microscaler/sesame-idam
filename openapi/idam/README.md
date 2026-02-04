# IDAM OpenAPI specs

OpenAPI specifications for the IDAM microservices. Each service has an **`openapi.yaml`** in its directory, derived from the BRRTRouter canonical specs.

| Service | Spec in this repo | Canonical (BRRTRouter) |
|---------|-------------------|------------------------|
| **Authentication (Identity)** | `authentication/openapi.yaml` | `docs/SPIFFY_mTLS/openapi/identity-openapi.yaml` |
| **Authorization (Access Management)** | `authorization/openapi.yaml` | `docs/SPIFFY_mTLS/openapi/access-management-openapi.yaml` |

- **Regenerate gen crates:** `just gen` or `just gen-auth` / `just gen-authorization` (requires BRRTRouter at `BRRTRouter_DIR` or `../BRRTRouter`).
- **Lint:** `just lint-openapi`.
- **Sync from canonical:** `just sync-specs-from-brrtrouter` (then restore the Sesame-IDAM header comment in each `openapi.yaml`).
