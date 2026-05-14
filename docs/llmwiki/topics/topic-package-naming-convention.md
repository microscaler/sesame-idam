---
title: Package Naming Convention
status: verified
updated: 2026-05-14
sources: [PRD-SEASAME-AUDIT-REMEDIATION.md, brrtrouter-workspace-architecture skill]
---

# Package Naming Convention

## Current State (Fixed)

All gen crate package names now match what impl crates expect. `cargo check --workspace` passes with 0 errors.

### Final Naming

| Service | Gen Package Name | Impl Package Name |
|---------|-----------------|-------------------|
| api-keys | `api_keys_service_api` | `sesame_idam_api_keys_gen_impl` |
| authz-core | `authz_core_service_api` | `sesame_idam_authz_core_gen_impl` |
| identity-login | `identity_login_service_service_api` | `sesame_idam_identity_login_service_gen_impl` |
| identity-session | `identity_session_service_service_api` | `sesame_idam_identity_session_service_gen_impl` |
| identity-user-mgmt | `identity_user_mgmt_service_service_api` | `sesame_idam_identity_user_mgmt_service_gen_impl` |
| org-mgmt | `org_mgmt_service_api` | `sesame_idam_org_mgmt_gen_impl` |

### How Naming Works

Gen crates declare `name = "<service>_service_api"` in `gen/Cargo.toml`. Impl crates declare a path dependency on the gen crate using the same name:

```toml
# impl/Cargo.toml
[dependencies]
identity_login_service_service_api = { path = "../gen" }
```

This matches because the gen crate was manually renamed from its short BRRTRouter default (e.g., `login_service`) to match the impl dependency name.

### Database Crate Naming

| Current | Target |
|---------|--------|
| `database` | `sesame_idam_database` |

Note: The database crate uses `sesame_idam_database` in dependency declarations but its own `Cargo.toml` still declares `name = "database"`. This works because path deps use the directory name, not the package name, for resolution.

## Code Anchors

- `PRD-SEASAME-AUDIT-REMEDIATION.md Section 4` — Full naming tables
- `PRD-SEASAME-AUDIT-REMEDIATION.md Section 5` — Phase 1 remediation plan
- `brrtrouter-workspace-architecture` skill — Package naming pitfall section
