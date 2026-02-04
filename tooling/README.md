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

The `sesame` CLI is a placeholder; codegen and OpenAPI lint are invoked via justfile (`just gen`, `just lint-openapi`) using BRRTRouter.
