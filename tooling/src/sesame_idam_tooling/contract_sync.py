"""Epic 15 contract lockstep: profile + OpenAPI + fixture versions."""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

import yaml

from sesame_idam_tooling.oidc_conformance import fixture_checksum, load_json, repo_root_from


def _read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def _required_fields(schema: dict[str, Any] | None) -> set[str]:
    if not schema:
        return set()
    required = schema.get("required") or []
    return {item for item in required if isinstance(item, str)}


def _validate_tenant_consumer_dogfood_shapes(openapi: dict[str, Any]) -> list[str]:
    """Series A P4 — public OpenAPI must match live register/invite/accept shapes."""
    problems: list[str] = []
    info = openapi.get("info") or {}
    if info.get("version") != "1.1.0":
        problems.append(
            f"tenant-consumer info.version={info.get('version')!r} expected '1.1.0'"
        )

    schemas = ((openapi.get("components") or {}).get("schemas")) or {}
    for name in (
        "RegisterRequest",
        "InvitationCreated",
        "OrganizationSummary",
        "InvitationPreview",
        "TokenResponse",
    ):
        if name not in schemas:
            problems.append(f"tenant-consumer missing schema {name}")

    register_required = _required_fields(schemas.get("RegisterRequest"))
    for field in ("client_id", "email", "password"):
        if field not in register_required:
            problems.append(f"RegisterRequest must require {field}")
    if register_required & {"first_name", "last_name"}:
        problems.append("RegisterRequest must not require first_name/last_name")

    invite_required = _required_fields(schemas.get("InvitationCreated"))
    for field in ("success", "invite_id", "invite_token"):
        if field not in invite_required:
            problems.append(f"InvitationCreated must require {field}")

    org_required = _required_fields(schemas.get("OrganizationSummary"))
    for field in ("id", "name", "tenant_id"):
        if field not in org_required:
            problems.append(f"OrganizationSummary must require {field}")

    token_required = _required_fields(schemas.get("TokenResponse"))
    for field in (
        "access_token",
        "expires_in",
        "token_type",
        "user_id",
        "refresh_token",
    ):
        if field not in token_required:
            problems.append(f"TokenResponse must require {field}")

    if "InvitationQueued" in schemas:
        problems.append("InvitationQueued was replaced by InvitationCreated")

    invite_ref = (
        ((openapi.get("paths") or {}).get("/organizations/{org_id}/invitations") or {})
        .get("post", {})
        .get("responses", {})
        .get("200", {})
        .get("content", {})
        .get("application/json", {})
        .get("schema", {})
        .get("$ref")
    )
    if invite_ref != "#/components/schemas/InvitationCreated":
        problems.append(
            "POST /organizations/{org_id}/invitations 200 must ref InvitationCreated"
        )

    return problems


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

    openapi_path = root / "openapi" / "idam" / "tenant-consumer" / "openapi.yaml"
    openapi_text = _read_text(openapi_path)
    profile_match = re.search(r'x-provider-profile:\s*"([^"]+)"', openapi_text)
    fixture_match = re.search(r'x-fixture-version:\s*"([^"]+)"', openapi_text)
    if not profile_match or profile_match.group(1) != manifest.get("provider_profile"):
        problems.append("tenant-consumer x-provider-profile out of sync with manifest")
    if not fixture_match or fixture_match.group(1) != manifest.get("fixture_version"):
        problems.append("tenant-consumer x-fixture-version out of sync with manifest")

    openapi_doc = yaml.safe_load(openapi_text)
    if not isinstance(openapi_doc, dict):
        problems.append("tenant-consumer openapi.yaml is not a mapping")
    else:
        problems.extend(_validate_tenant_consumer_dogfood_shapes(openapi_doc))

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
