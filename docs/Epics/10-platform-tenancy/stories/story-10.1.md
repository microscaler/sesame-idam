# Story 10.1: Platform OpenAPI Spec + Codegen

## Epic

[10-platform-tenancy](../README.md) · [PRD-P1](../../../PRD-P1-platform-tenant-admin.md)

## Summary

Add `PlatformAdmin` tag and platform tenant paths to `identity-login-service` OpenAPI; run codegen; register handler stubs.

## Acceptance Criteria

- [ ] `openapi/idam/identity-login-service/openapi.yaml` includes tag `PlatformAdmin`
- [ ] Schemas: `PlatformTenant`, `PlatformTenantCreate`, `PlatformTenantStatusPatch`, `TenantOAuthConfig`, `OAuthRotateRequest`
- [ ] Security scheme `PlatformServiceAuth` (`X-Platform-Admin-Key`); platform paths use it explicitly (override global Bearer)
- [ ] Paths defined: `POST/GET /platform/tenants`, `PATCH /platform/tenants/{slug}/status`, `PUT/POST oauth` per PRD §6
- [ ] `just gen-identity-login` succeeds; `just lint-openapi` clean
- [ ] Gen registry includes new handlers; impl `platform_*` controller modules stubbed

## Implementation Notes

- Base path: `/idam/v1` (existing servers block)
- `slug` path param: `pattern: '^[a-z][a-z0-9-]{2,63}$'`
- `provider` enum: `google`, `microsoft`
- Do not edit `gen/` by hand — spec only

## Dependencies

- ADR-004 models (done)

## Tests

- OpenAPI lint passes
- Smoke: generated registry lists all 5 platform handler names
