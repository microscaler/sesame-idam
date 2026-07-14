# Story 10.5: OAuth Rotation + Audit (POST)

## Epic

[10-platform-tenancy](../README.md) · [PRD-P1](../../../PRD-P1-platform-tenant-admin.md)

## Summary

Implement `POST /platform/tenants/{slug}/oauth/{provider}/rotate` using `TenantOAuthService::record_rotation`.

## Acceptance Criteria

- [ ] Increments `config_version` by 1
- [ ] Sets `last_rotated_at`, `last_rotated_by` from request body
- [ ] Response `{ "config_version": N }`
- [ ] Missing oauth row → `404 oauth_config_not_found`
- [ ] Audit event emitted (extend `AuditEventType` with `OAuthCredentialRotated` or reuse structured `decision_source`)
- [ ] Audit payload: `tenant_slug`, `provider`, `config_version` — no secret

## Implementation Notes

- Controller: `platform_tenant_oauth_rotate.rs`
- Rotation assumes secret updated out-of-band in K8s

## Dependencies

- 10.4

## Tests

- BDD: create oauth row → rotate → GET shows `config_version=2` and `last_rotated_by` set
- Unit: double rotate bumps version twice
