"""Epic 15 contract lockstep: profile + OpenAPI + fixture versions."""

from __future__ import annotations

import json
import re
from pathlib import Path

from sesame_idam_tooling.oidc_conformance import fixture_checksum, load_json, repo_root_from


def _read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def validate_contract_sync(root: Path | None = None) -> list[str]:
    root = root or repo_root_from()
    problems: list[str] = []

    version_path = root / "conformance" / "oidc-v1" / "VERSION"
    if not version_path.is_file():
        return ["conformance/oidc-v1/VERSION missing"]
    version_doc = {}
    for line in _read_text(version_path).splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        version_doc[key.strip()] = value.strip()

    manifest = load_json(root / "conformance" / "oidc-v1" / "manifest.json")
    for key in ("provider_profile", "fixture_version"):
        if version_doc.get(key) != manifest.get(key):
            problems.append(
                f"VERSION {key}={version_doc.get(key)!r} != manifest {manifest.get(key)!r}"
            )

    checksum_path = root / "conformance" / "oidc-v1" / "CHECKSUM"
    if checksum_path.is_file():
        recorded = _read_text(checksum_path).strip().split()[0]
        actual = fixture_checksum(root)
        if recorded != actual:
            problems.append(f"CHECKSUM mismatch: recorded={recorded} actual={actual}")
    else:
        problems.append("conformance/oidc-v1/CHECKSUM missing")

    openapi = _read_text(root / "openapi" / "idam" / "tenant-consumer" / "openapi.yaml")
    profile_match = re.search(r'x-provider-profile:\s*"([^"]+)"', openapi)
    fixture_match = re.search(r'x-fixture-version:\s*"([^"]+)"', openapi)
    if not profile_match or profile_match.group(1) != manifest.get("provider_profile"):
        problems.append("tenant-consumer x-provider-profile out of sync with manifest")
    if not fixture_match or fixture_match.group(1) != manifest.get("fixture_version"):
        problems.append("tenant-consumer x-fixture-version out of sync with manifest")

    schema = json.loads(
        _read_text(
            root / "docs" / "standards-first-oidc" / "verified-principal-v1.schema.json"
        )
    )
    const_version = (schema.get("properties") or {}).get("profile_version", {}).get("const")
    if const_version != manifest.get("provider_profile"):
        problems.append(
            f"verified-principal profile_version const {const_version!r} "
            f"!= provider_profile {manifest.get('provider_profile')!r}"
        )

    return problems


def main() -> int:
    problems = validate_contract_sync()
    digest = fixture_checksum()
    print(f"fixture_checksum={digest}")
    if problems:
        print("FAIL contract sync:")
        for problem in problems:
            print(f"  - {problem}")
        return 1
    print("OK contract sync")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
