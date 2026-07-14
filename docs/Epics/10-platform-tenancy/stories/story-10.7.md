# Story 10.7: Platform Service Authentication

## Epic

[10-platform-tenancy](../README.md) · [PRD-P1](../../../PRD-P1-platform-tenant-admin.md)

## Summary

Protect all `/platform/*` routes with `X-Platform-Admin-Key`; reject end-user Bearer tokens.

## Acceptance Criteria

- [ ] Missing header → `401 unauthorized`
- [ ] Wrong key → `401 unauthorized`
- [ ] Valid platform key → request proceeds
- [ ] Valid user Bearer JWT without platform key → `403 forbidden`
- [ ] Key read from env `SESAME_PLATFORM_ADMIN_KEY` at startup; empty key → platform routes return `503 platform_auth_unconfigured` in dev with clear log
- [ ] OpenAPI documents `PlatformServiceAuth` security requirement on platform paths only

## Implementation Notes

- Middleware or per-handler check in BRRTRouter impl layer
- Consider timing-safe comparison for key (`subtle::ConstantTimeEq` pattern in Rust)
- Dev Kind: document key in Tilt `database-env` or sealed secret template

## Dependencies

- 10.1 (security scheme in spec)

## Tests

- BDD/security: each platform route without key → 401
- User JWT on platform route → 403
- Valid key → 201 on create (with 10.2)

## Security

- Never log the platform admin key
- Rotate key via K8s secret reload (document in runbook)
