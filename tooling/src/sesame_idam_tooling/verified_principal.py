"""Verified-principal mapping + JSON Schema checks (Epic 15.2 / 15.8)."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from sesame_idam_tooling.oidc_conformance import repo_root_from

PROFILE_VERSION = "1.0.0"
SX_NS = "https://sesame-idam.dev/claims"


def load_schema(root: Path | None = None) -> dict[str, Any]:
    root = root or repo_root_from()
    path = root / "docs" / "standards-first-oidc" / "verified-principal-v1.schema.json"
    return json.loads(path.read_text(encoding="utf-8"))


def map_access_claims_to_principal(claims: dict[str, Any]) -> dict[str, Any]:
    """Map access-token claim dict → verified principal (post-validation)."""
    sx = claims.get(SX_NS) or {}
    roles = sorted(set(sx.get("roles") or []))
    permissions = sorted(set(sx.get("permissions") or []))
    org = claims.get("org_id")
    principal: dict[str, Any] = {
        "profile_version": PROFILE_VERSION,
        "tenant_id": claims["tenant_id"],
        "subject": claims["sub"],
        "client_id": claims["client_id"],
        "application_id": claims["client_id"],
        "session_id": claims["sid"],
        "token_version": int(claims["ver"]),
        "organization_id": org if org is not None else None,
        "user_type": claims["user_type"],
        "roles": roles,
        "permissions": permissions,
        "entitlements_ref": sx.get("entitlements_ref"),
        "entitlements_hash": sx.get("entitlements_hash"),
        "actor": claims.get("act"),
    }
    portal = sx.get("portal")
    if portal:
        principal["portal"] = portal
    return principal


def _type_ok(value: Any, declared: Any) -> bool:
    if isinstance(declared, list):
        return any(_type_ok(value, item) for item in declared)
    if declared == "string":
        return isinstance(value, str)
    if declared == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    if declared == "array":
        return isinstance(value, list)
    if declared == "object":
        return isinstance(value, dict)
    if declared == "null":
        return value is None
    return True


def validate_principal(principal: dict[str, Any], schema: dict[str, Any] | None = None) -> list[str]:
    """Lightweight draft-2020-12 subset validator for the verified-principal schema."""
    schema = schema or load_schema()
    problems: list[str] = []
    required = schema.get("required") or []
    properties = schema.get("properties") or {}

    for key in required:
        if key not in principal:
            problems.append(f"missing required field {key}")

    if schema.get("additionalProperties") is False:
        for key in principal:
            if key not in properties:
                problems.append(f"unexpected field {key}")

    for key, prop in properties.items():
        if key not in principal:
            continue
        value = principal[key]
        if "const" in prop and value != prop["const"]:
            problems.append(f"{key} must be {prop['const']!r}")
        if "type" in prop and not _type_ok(value, prop["type"]):
            problems.append(f"{key} has wrong type")
        if prop.get("type") == "array" and isinstance(value, list):
            if prop.get("uniqueItems") and len(value) != len(set(value)):
                problems.append(f"{key} must have unique items")
            for item in value:
                if not isinstance(item, str):
                    problems.append(f"{key} items must be strings")
                    break
        if key == "token_version" and isinstance(value, int) and value < 1:
            problems.append("token_version must be >= 1")
    return problems


def sample_access_claims(*, with_org: bool = False) -> dict[str, Any]:
    claims: dict[str, Any] = {
        "sub": "11111111-1111-1111-1111-111111111111",
        "user_id": "11111111-1111-1111-1111-111111111111",
        "client_id": "acme-web",
        "sid": "sid-1",
        "ver": 1,
        "tenant_id": "acme",
        "user_type": "customer",
        SX_NS: {
            "tenant": "acme",
            "portal": "acme-web",
            "roles": ["owner"],
            "permissions": ["org:admin"],
        },
    }
    if with_org:
        claims["org_id"] = "22222222-2222-2222-2222-222222222222"
    return claims
