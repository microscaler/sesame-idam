# Story 10.8: BDD — Platform Mint to End-User Auth

## Epic

[10-platform-tenancy](../README.md) · [PRD-P1](../../../PRD-P1-platform-tenant-admin.md)

## Summary

End-to-end BDD proving platform-created tenant supports register/login; suspend blocks auth.

## Acceptance Criteria

- [ ] Test: `POST /platform/tenants` (or `ensure` via service layer in test helper) creates new slug → `POST /auth/register` → `201` → `POST /auth/login` → `200`
- [ ] Test: slug not in registry before create → register → `404 tenant_unknown`
- [ ] Test: suspend via PATCH → login → `403 tenant_not_active`
- [ ] Test: OAuth rotate via API → `config_version` visible on subsequent GET
- [ ] Tests skip gracefully without Postgres (existing pattern)
- [ ] `just nt` / `cargo nextest run -p sesame_idam_identity_login_service` green on ms02

## Implementation Notes

- File: `impl/tests/bdd/platform_tenant_admin.rs`
- Use platform admin key in test env `TEST_PLATFORM_ADMIN_KEY`
- May call controllers directly (like other BDD) or HTTP if TestClient available

## Dependencies

- 10.2, 10.3, 10.5, 10.7

## Tests

This story **is** the integration test suite for P1.
