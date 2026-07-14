# Story 10.6: Platform Tenant CLI

## Epic

[10-platform-tenancy](../README.md) · [PRD-P1](../../../PRD-P1-platform-tenant-admin.md)

## Summary

Add `sesame-idam tenant` subcommands in `tooling/` that call platform REST API via HTTP (CLI-first — no direct DB).

## Acceptance Criteria

- [ ] Subcommands per PRD FR-P1-008: `create`, `get`, `status set`, `oauth set`, `oauth rotate`
- [ ] Uses `may_http` equivalent in Python: `httpx` or stdlib `urllib` (follow tooling conventions)
- [ ] Env: `SESAME_LOGIN_SERVICE_URL` (default `http://127.0.0.1:8101/idam/v1` dev), `SESAME_PLATFORM_ADMIN_KEY`
- [ ] Exit code non-zero on API error; prints JSON error body
- [ ] `just qa` passes (ruff on new module)
- [ ] README snippet in `tooling/README.md`

## Implementation Notes

- Module: `tooling/src/sesame_idam_tooling/cli/tenant.py` (or `platform_tenant.py`)
- Register in `cli/main.py` argparse tree
- No shell scripts

## Dependencies

- 10.2–10.5 (API live)

## Tests

- Unit tests with mocked HTTP responses
- Manual smoke on ms02 against port-forwarded login-service
