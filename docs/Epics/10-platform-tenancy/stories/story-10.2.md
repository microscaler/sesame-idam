# Story 10.2: Create and Get Platform Tenant

## Epic

[10-platform-tenancy](../README.md) · [PRD-P1](../../../PRD-P1-platform-tenant-admin.md)

## Summary

Implement `POST /platform/tenants` and `GET /platform/tenants/{slug}` using `TenantService`.

## Acceptance Criteria

- [ ] `POST` creates row with `provisioning_mode=platform` (default); `self_service` rejected with `400`
- [ ] `activate=true` (default) → `status=active`; `activate=false` → `status=provisioning`
- [ ] Duplicate slug → `409 slug_taken`
- [ ] Invalid slug format → `400 invalid_slug`
- [ ] Reserved slugs (`admin`, `platform`, `www`, `api`, `idam`) → `400 reserved_slug`
- [ ] `GET` returns tenant + oauth provider list (metadata only)
- [ ] Unknown slug on GET → `404 tenant_not_found`
- [ ] Platform auth enforced (story 10.7)

## Implementation Notes

- Controllers: `platform_tenant_create.rs`, `platform_tenant_get.rs`
- Reuse `TenantService::create`; add `find_by_slug` for GET
- Extend `TenantService` with `set_status` if needed for 10.3

## Dependencies

- 10.1 (codegen), 10.7 (auth)

## Tests

- Unit: slug validation, reserved list
- BDD: POST create → GET returns same slug and `active`
