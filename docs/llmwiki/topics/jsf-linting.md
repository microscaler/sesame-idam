---
title: JSF-Inspired Linting
status: verified
updated: 2026-07-13
sources: [clippy.toml, justfile (lint-rust), BRRTRouter docs/JSF/]
---

# JSF-Inspired Linting

## Overview

Sesame-IDAM uses a JSF-inspired clippy profile aligned with BRRTRouter and lifeguard. The configuration enforces strict error handling and bounded complexity, adapted from the [JSF AV C++ coding rules](https://www.stroustrup.com/JSF-AV-rules.pdf) for Rust.

## Configuration Files

| File | Purpose |
|------|---------|
| `clippy.toml` | JSF-aligned numeric thresholds (shared with BRRTRouter / lifeguard) |
| `justfile` → `lint-rust` | Command that runs clippy with pedantic mode |

## Command

```
just lint-rust
```

Runs `cargo clippy --all-targets --all-features --no-deps` with `-D warnings -W clippy::pedantic` on the six implementation crates, database, and migrator. Generated crates are excluded. `sesame-common` is intentionally checked by the separate warning-only `just lint-common` recipe while its documented pedantic backlog is reduced.

The 2026-07-13 delivered-code audit restored this gate after it exposed stale `TypedHandlerRequest` fixtures, generated-code lint leakage in org-mgmt, and mechanical pedantic failures across production and test targets.

## JSF-Aligned Thresholds (clippy.toml)

| Threshold | Value | Rationale |
|-----------|-------|-----------|
| `stack-size-threshold` | 512000 | Warn on large stack allocations |
| `enum-variant-size-threshold` | 256 | Allow reasonable size diffs in routing enums |
| `type-complexity-threshold` | 300 | Hot path type complexity limit |
| `cognitive-complexity-threshold` | 30 | Functions should be simple (JSF: bounded complexity) |
| `missing-docs-in-crate-items` | false | Permissive for generated-adjacent code |
| `too-many-arguments-threshold` | 8 | Handler argument count limit |
| `too-many-lines-threshold` | 200 | Function length limit |
| `single-char-binding-names-threshold` | 4 | Closure binding name length |

## Pedantic Mode

Pedantic mode (`-W clippy::pedantic`) is **mandatory** for all Sesame-IDAM code. It is a security-critical project and pedantic catches issues that basic clippy misses:

- Missing `#[must_use]` on functions that return `Result`
- Needless `return` statements
- Explicit `Iterator::map` followed by `.collect()` that could use `.into_iter()`
- Missing documentation on public items
- Unnecessary `self` borrows

## Lint Categories (Phase 1 — warn, not deny)

From BRRTRouter's JSF lint profile, adapted for sesame-idam:

### Panic Prevention (JSF AV Rule 208 adaptation)
- `unwrap_used` → warn
- `expect_used` → warn
- `panic` → warn
- `unreachable` → warn

### Allocation Discipline (JSF AV Rule 206 adaptation)
- Hot paths should minimize allocations for predictable latency
- Currently informational — not enforced

### Error Handling Hygiene
- `let_underscore_must_use` → warn
- `must_use_candidate` → warn

### Code Quality
- `clone_on_ref_ptr` → warn (clone on `&Arc`/`&Rc` is usually a mistake)
- `redundant_clone` → warn
- `large_futures` → warn (futures too large for the stack)
- `large_stack_arrays` → warn

## Future Work (Phase 2 — deny)

The goal is to graduate from warn to deny on critical lints as the codebase stabilizes:

- `unwrap_used` → deny (eliminate unwrap in production code)
- `expect_used` → deny
- `panic` → deny in hot paths

## References

- BRRTRouter: `docs/JSF/JSF_WRITEUP.md` — full JSF adaptation document
- BRRTRouter: `docs/JSF/JSF_AUDIT_OPINION.md` — audit opinion on the adaptation
- BRRTRouter: `clippy.toml` — shared thresholds
- BRRTRouter: `AGENTS.md` → "Hot-path JSF-AV safety"
- JSF AV Rules: [JSF-AV-rules.pdf](https://www.stroustrup.com/JSF-AV-rules.pdf)
