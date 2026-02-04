# Authorization (Access Management) API

**Spec:** `openapi.yaml` in this directory (derived from BRRTRouter canonical `docs/SPIFFY_mTLS/openapi/access-management-openapi.yaml`).

Covers: applications, roles, permissions, role–permission links, principal–role/attribute assignments, `principal/effective`, `authorize`. Dot-notation app slugs; qualified roles/permissions.

- **Regenerate from this spec:** `just gen-authorization` (requires BRRTRouter at `BRRTRouter_DIR` or `../BRRTRouter`).
- **Sync from canonical:** `just sync-specs-from-brrtrouter` then restore the Sesame-IDAM header comment in `openapi.yaml`.
