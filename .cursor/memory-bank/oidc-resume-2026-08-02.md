# OIDC / PRD v2 resume — 2026-08-03

## Implemented this session (uncommitted unless noted)

### sesame-idam P0–P2
- Invite JWT-bound (`invite_user_to_org` typed + admin)
- List members RLS session (`with_pre_auth_tenant`)
- `sesame_common::smtp` + invite email send
- Helm `SMTP_*` + `INVITE_MAGIC_LINK_BASE`
- org-mgmt image rebuilt/published; pod rolled (~P0/P1); rebuild again for SMTP

### sesame-idam-client
- Bearer routes omit `X-Tenant-ID` — **commit/push + bump Hauliage pin** still needed
- Public edge already strips header, so platform fix unblocks dogfood without client bump

### hauliage
- BFF `org_error_response` richer upstream fields

## Still open
- P3 accept-invite + active-org hardening
- P4 tenant-consumer OpenAPI sync
- Hauliage P0 hydrate/onboarding polish + redeploy BFF after client pin
- Commit/push across repos when user asks
