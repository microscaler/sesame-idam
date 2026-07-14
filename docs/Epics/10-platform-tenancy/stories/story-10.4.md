# Story 10.4: OAuth Metadata Upsert (PUT)

## Epic

[10-platform-tenancy](../README.md) · [PRD-P1](../../../PRD-P1-platform-tenant-admin.md)

## Summary

Implement `PUT /platform/tenants/{slug}/oauth/{provider}` delegating to `TenantOAuthService::upsert_metadata`.

## Acceptance Criteria

- [ ] Upsert creates or updates `tenant_oauth_providers` row
- [ ] `redirect_uris` accepted as JSON array; stored comma-separated internally
- [ ] Response includes `config_version`; never includes secret value
- [ ] Tenant `deprovisioned` → `409 tenant_deprovisioned`
- [ ] Unknown provider string → `400 unsupported_provider`
- [ ] Unknown tenant → `404 tenant_not_found`
- [ ] `enabled=false` disables provider for `TenantOAuthService::resolve`

## Implementation Notes

- Controller: `platform_tenant_oauth_upsert.rs`
- Service already exists — wire controller only
- Document ops runbook: K8s secret must exist before social login works

## Dependencies

- 10.2, 10.7

## Tests

- Unit: redirect URI serialization
- BDD: PUT metadata → social_login returns `503 oauth_not_configured` without env secret; with env secret set → redirect succeeds
