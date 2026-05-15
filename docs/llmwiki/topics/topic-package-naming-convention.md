---
title: Package Naming Convention
status: verified
updated: 2026-05-15
sources: [PRD-SEASAME-AUDIT-REMEDIATION.md, brrtrouter-workspace-architecture skill, actual cargo check output]
---

# Package Naming Convention

## Current State (Fixed — Post Phase 0)

All gen and impl package names now match. `cargo check --workspace` passes with 0 errors across all 6 services.

### Final Naming (Verified 2026-05-15)

| Service | Gen Package Name | Impl Package Name | Impl Binary Name |
|---------|-----------------|-------------------|------------------|
| api-keys | `sesame_idam_api_keys_gen` | `sesame_idam_api_keys` | `sesame_idam_api_keys` |
| authz-core | `sesame_idam_authz_core_gen` | `sesame_idam_authz_core` | `sesame_idam_authz_core` |
| identity-login-service | `sesame_idam_identity_login_service_gen` | `sesame_idam_identity_login_service` | `sesame_idam_identity_login_service` |
| identity-session-service | `sesame_idam_identity_session_service_gen` | `sesame_idam_identity_session_service` | `sesame_idam_identity_session_service` |
| identity-user-mgmt-service | `sesame_idam_identity_user_mgmt_service_gen` | `sesame_idam_identity_user_mgmt_service` | `sesame_idam_identity_user_mgmt_service` |
| org-mgmt | `sesame_idam_org_mgmt_gen` | `sesame_idam_org_mgmt` | `sesame_idam_org_mgmt` |

### How Naming Works

Gen crates declare `name = "sesame_idam_<svc>_gen"` in `gen/Cargo.toml`. Impl crates declare a path dependency on the gen crate using the same name:

```toml
# impl/Cargo.toml
[dependencies]
sesame_idam_identity_login_service_gen = { path = "../gen" }
```

The `[[bin]]` `name` field matches `[package].name` in every impl crate. This is critical — Tilt's `get_package_name()` reads the package name and appends it to the artifact path. Mismatched names cause `❌ Artifact not found` errors.

### Database Crate Naming

| Name | Value |
|------|-------|
| `Cargo.toml` name | `sesame_idam_database` |
| Workspace member path | `database` |
| Dependency in impl crates | `sesame_idam_database = { path = "../../../database" }` |

Path deps use the directory name (`../../../database`) for resolution, not the package name.

## Code Anchors

- `PRD-SEASAME-AUDIT-REMEDIATION.md Section 4` — Full naming tables
- `PRD-SEASAME-AUDIT-REMEDIATION.md Section 5` — Phase 1 remediation plan
- `brrtrouter-workspace-architecture` skill — Package naming pitfall section
- All `microservices/idam/<svc>/gen/Cargo.toml` — `[package].name` declarations
- All `microservices/idam/<svc>/impl/Cargo.toml` — `[package].name`, `[[bin]]`, `[dependencies]`
