#!/usr/bin/env python3
"""Fail the build on an endpoint that declares no authentication without saying why.

WHY THIS EXISTS
---------------
`security: []` in OpenAPI is not "inherit the default" — it is the explicit
spelling of *no authentication required*, and it silently overrides the
document-level `security` block.

All 11 authz-core operations carried it. Nobody noticed until an unrelated
audit, by which point an unauthenticated caller could export any tenant's audit
trail and delete the retention policy governing how long that evidence is kept
(FINDING-2026-07-25-authz-core-unauthenticated.md).

Being public is sometimes correct — a login endpoint must be. The failure was
that "correct" and "forgotten" looked identical in the file. This lint makes
them different: a deliberately public operation carries `x-public-reason`
explaining itself, and an undeclared one fails CI.

It is deliberately dumb. It does not decide whether the reason is *good*; a
human does that at review. It only guarantees somebody was asked.

USAGE
-----
    scripts/lint-public-endpoints.py [spec-root]

Exit 0 when every public operation is annotated, 1 otherwise.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

# Matches `  operationId: foo` at any indent.
OPERATION_ID = re.compile(r"^(\s*)operationId:\s*(\S+)\s*$")
# `security: []` — the explicit no-auth declaration.
NO_AUTH = re.compile(r"^\s*security:\s*\[\s*\]\s*$")
# The annotation that makes it deliberate.
REASON = re.compile(r"^\s*x-public-reason:\s*(\S.*)$")


def audit_spec(path: Path) -> tuple[list[str], list[tuple[str, str]]]:
    """Return (unannotated operation ids, [(op, reason)] for annotated ones).

    The reason is searched for within the operation's own block: from its
    `operationId` to the next one. Scoping it that way stops an annotation on
    one operation from silently excusing its neighbour.
    """
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()

    # Index where each operation's block starts, in file order.
    starts: list[tuple[int, str]] = []
    for i, line in enumerate(lines):
        m = OPERATION_ID.match(line)
        if m:
            starts.append((i, m.group(2)))

    unannotated: list[str] = []
    annotated: list[tuple[str, str]] = []

    for idx, (start, op) in enumerate(starts):
        end = starts[idx + 1][0] if idx + 1 < len(starts) else len(lines)
        block = lines[start:end]
        if not any(NO_AUTH.match(l) for l in block):
            continue
        reason = next((REASON.match(l).group(1) for l in block if REASON.match(l)), None)
        if reason:
            annotated.append((op, reason.strip().strip('"')))
        else:
            unannotated.append(op)

    return unannotated, annotated


def main(argv: list[str]) -> int:
    root = Path(argv[1]) if len(argv) > 1 else Path(__file__).resolve().parent.parent / "openapi"
    specs = sorted(root.rglob("openapi.yaml"))
    if not specs:
        print(f"lint-public-endpoints: no specs found under {root}", file=sys.stderr)
        return 1

    total_bad = 0
    total_ok = 0

    for spec in specs:
        # Generated copies mirror the source; linting both doubles every message.
        if "/gen/" in str(spec):
            continue
        unannotated, annotated = audit_spec(spec)
        total_ok += len(annotated)
        rel = spec.relative_to(root) if root in spec.parents else spec
        if annotated:
            print(f"  ok  {rel}: {len(annotated)} public, all annotated")
            for op, reason in annotated:
                print(f"        {op}: {reason}")
        if unannotated:
            total_bad += len(unannotated)
            print(f"FAIL  {rel}: {len(unannotated)} public operation(s) with no x-public-reason")
            for op in unannotated:
                print(f"        {op}")

    print()
    if total_bad:
        print(f"{total_bad} operation(s) declare `security: []` without saying why.")
        print()
        print("`security: []` means NO AUTHENTICATION — it overrides the document-level")
        print("security block. If that is intended, say so on the operation:")
        print()
        print('    x-public-reason: "login must be reachable before a token exists"')
        print()
        print("If it is not intended, give the operation a real security scheme. Check")
        print("which schemes the service actually registers at startup first: declaring")
        print("an unconfigured one can fall back to a static key and look fixed.")
        return 1

    print(f"All {total_ok} public operation(s) annotated.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
