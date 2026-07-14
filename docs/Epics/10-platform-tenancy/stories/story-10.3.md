# Story 10.3: Tenant Status Lifecycle (PATCH)

## Epic

[10-platform-tenancy](../README.md) · [PRD-P1](../../../PRD-P1-platform-tenant-admin.md)

## Summary

Implement `PATCH /platform/tenants/{slug}/status` with validated transitions and audit.

## Acceptance Criteria

- [ ] Allowed transitions per PRD FR-P1-004; invalid → `409 invalid_status_transition`
- [ ] `suspended` tenant: `POST /auth/login` returns `403 tenant_not_active` (existing gate)
- [ ] `deprovisioned` tenant: same as suspended for auth
- [ ] `provisioning` → `active` enables auth (BDD with register)
- [ ] Audit event `tenant_status_changed` with `tenant_slug`, `old_status`, `new_status`
- [ ] Unknown slug → `404 tenant_not_found`

## Implementation Notes

- `TenantService::transition_status(slug, new_status)` centralizes rules
- Use `TenantRecord` update pattern (lifeguard)

## Dependencies

- 10.2

## Tests

- Unit: full transition matrix (allow + deny cases)
- BDD: create `provisioning` → register fails `403` → PATCH active → register succeeds
- BDD: suspend → login `403`
