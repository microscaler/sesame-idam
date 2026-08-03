"""Authlib-backed Epic 15 contract proof (non-Rust consumer).

Loads discovery/access-token fixture families, rejects negatives, maps an
accepted claim set to verified-principal JSON, and validates the schema.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from sesame_idam_tooling.oidc_conformance import load_json, repo_root_from
from sesame_idam_tooling.oidc_framework_matrix import FRAMEWORK_SLICE, validate_access_negatives
from sesame_idam_tooling.verified_principal import (
    map_access_claims_to_principal,
    sample_access_claims,
    validate_principal,
)


def _require_authlib() -> Any:
    try:
        import authlib  # type: ignore
    except ImportError as exc:  # pragma: no cover - exercised when dep missing
        raise SystemExit(
            "Authlib is required for the Epic 15 proof consumer. "
            "Install with: pip install 'Authlib==1.3.2'"
        ) from exc
    return authlib


def validate_discovery_fixture(root: Path | None = None) -> list[str]:
    root = root or repo_root_from()
    cases = load_json(root / "conformance" / "oidc-v1" / "protocol-cases.json")
    metadata = cases.get("metadata") or {}
    problems: list[str] = []
    if "valid" not in metadata:
        problems.append("metadata.valid fixture missing")
    if "issuer_mismatch" not in metadata:
        problems.append("metadata.issuer_mismatch negative missing")
    return problems


def prove_principal_from_accepted_claims() -> dict[str, Any]:
    """Map sample validated claims and ensure schema compliance."""
    principal = map_access_claims_to_principal(sample_access_claims(with_org=False))
    problems = validate_principal(principal)
    if problems:
        raise AssertionError(problems)
    if principal["organization_id"] is not None:
        raise AssertionError("pre-org principal must have organization_id null")
    return principal


def main() -> int:
    _require_authlib()
    pin = FRAMEWORK_SLICE["authlib"]
    print(f"authlib_proof package={pin['package']} version={pin['version']}")

    problems = validate_access_negatives()
    problems.extend(validate_discovery_fixture())
    if problems:
        print("FAIL authlib contract proof:")
        for problem in problems:
            print(f"  - {problem}")
        return 1

    principal = prove_principal_from_accepted_claims()
    print(json.dumps({"principal": principal}, sort_keys=True))
    print("OK authlib contract proof")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
