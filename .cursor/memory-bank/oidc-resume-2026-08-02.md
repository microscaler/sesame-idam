# Owner transfer dual-factor + role PATCH fix — 2026-08-03

## Role change bug (fixed)
- PATCH role returned Sesame 500 Response validation failed (`"error" is a required property`).
- Cause: 200 had no schema → defaulted to ErrorResponse; success body `{}` invalid.
- Fix: `ChangeUserRoleResponse` + controller returns org_id/user_id/primary_role.
- Also loosened ErrorResponse.error enum for membership codes.

## Owner transfer policy (locked)
- Role elevate: UI confirm only.
- Owner transfer product path: **password + email OTP** (dual-factor).
- CS path: TenantCsAuth + reason (no password/OTP).
- v2: prefer TOTP/WebAuthn + SMS when enrolled/verified.

## Still open
- Wait for Tilt: org-mgmt + bff + frontend; dogfood role change then transfer.
- Commit when asked.
