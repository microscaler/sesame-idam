# Sesame-IDAM tooling

Same strict **ruff** and guard rails as [RERP](https://github.com/microscaler/rerp) tooling:

- **Ruff:** E, F, W, B, C4, UP, SIM, I, PTH, RUF, S110, A, BLE, ERA, T10, EM, RET, LOG, PIE, PT, RSE, PGH, N, PERF, C90, TRY (mccabe max-complexity 20).
- **Pre-commit:** `just qa` (lint + format-check + pytest) and check for forbidden empty print statements.
- **Justfile:** `just init`, `just lint`, `just format`, `just format-check`, `just qa`, `just install-hooks`, `just lint-fix`, `just lint-unused-imports`.

Run from repo root:

```bash
just init          # Once: create tooling/.venv, install sesame-idam-tooling[dev]
just qa            # Lint + format-check + pytest (same as pre-commit)
just install-hooks # Install pre-commit hooks
```

### Platform tenant CLI (P1)

```bash
export SESAME_PLATFORM_ADMIN_KEY=dev-platform-admin
export SESAME_LOGIN_SERVICE_URL=http://127.0.0.1:8101/idam/v1

sesame-idam tenant create --slug pricewhisperer --display-name "PriceWhisperer"
sesame-idam tenant get --slug pricewhisperer
sesame-idam tenant status set --slug bad-actor --status suspended
sesame-idam tenant oauth set --slug pricewhisperer --provider google \
  --client-id "$CLIENT_ID" \
  --redirect-uris "http://localhost/oauth/callback" \
  --secret-env-key SESAME_OAUTH__PRICEWHISPERER__GOOGLE_CLIENT_SECRET
sesame-idam tenant oauth rotate --slug pricewhisperer --provider google --by ops@example.com
```

The `sesame-idam` CLI delegates workspace commands to BRRTRouter; `tenant` subcommands call
identity-login-service platform REST API over HTTP (no direct DB).
