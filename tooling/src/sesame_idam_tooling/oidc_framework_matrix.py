"""Framework matrix helpers for Epic 14.7 first slice.

Records pinned library versions and validates that the shared access-token
negative fixture family is present for consumer projects to exercise.
"""

from __future__ import annotations

import json
from pathlib import Path

from sesame_idam_tooling.oidc_conformance import load_json, repo_root_from

# First-slice pins (also recorded in evidence/framework-matrix-v1.md)
FRAMEWORK_SLICE = {
    "authjs": {
        "package": "@auth/core",
        "version": "5.0.0-beta.25",
        "role": "rp",
    },
    "authlib": {
        "package": "Authlib",
        "version": "1.3.2",
        "role": "rp",
    },
    "spring_resource_server": {
        "package": "spring-security-oauth2-jose",
        "version": "6.3.3",
        "role": "resource-server",
    },
}

REQUIRED_ACCESS_NEGATIVES = {
    "wrong_issuer",
    "wrong_audience",
    "alg_none",
    "expired",
    "tenant_mismatch",
}


def validate_access_negatives(root: Path | None = None) -> list[str]:
    root = root or repo_root_from()
    cases = load_json(root / "conformance" / "oidc-v1" / "protocol-cases.json")
    access = cases.get("access_token") or {}
    missing = sorted(REQUIRED_ACCESS_NEGATIVES - set(access))
    if missing:
        return [f"access_token negatives missing: {', '.join(missing)}"]
    return []


def matrix_document() -> dict:
    return {
        "slice": "epic-14.7-first-cut",
        "frameworks": FRAMEWORK_SLICE,
        "negative_fixture_family": "access_token",
        "evidence": "docs/Epics/14-oidc-security-conformance/evidence/framework-matrix-v1.md",
    }


def main() -> int:
    problems = validate_access_negatives()
    doc = matrix_document()
    print(json.dumps(doc, indent=2, sort_keys=True))
    if problems:
        for problem in problems:
            print(f"FAIL: {problem}")
        return 1
    print("OK framework matrix fixture binding")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
