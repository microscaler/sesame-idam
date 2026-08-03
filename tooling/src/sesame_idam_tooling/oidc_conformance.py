"""OIDC conformance fixture gate helpers (Epic 14.9).

Validates the language-neutral corpus under ``conformance/oidc-v1/`` and
computes a stable checksum used by the CI release gate.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path


REQUIRED_MANIFEST_IDS = {
    "metadata-valid",
    "authorization-valid-public-pkce",
    "authorization-redirect-prefix",
    "authorization-pkce-plain",
    "code-replay",
    "code-cross-client",
    "refresh-rotation",
    "refresh-replay",
    "refresh-cross-client",
    "access-alg-none",
    "userinfo-substitution",
}


def repo_root_from(start: Path | None = None) -> Path:
    here = (start or Path(__file__)).resolve()
    for candidate in [here, *here.parents]:
        if (candidate / "conformance" / "oidc-v1" / "manifest.json").is_file():
            return candidate
    raise FileNotFoundError("conformance/oidc-v1/manifest.json not found")


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def fixture_checksum(root: Path | None = None) -> str:
    """SHA-256 over canonical JSON of manifest + protocol-cases."""
    root = root or repo_root_from()
    corpus = root / "conformance" / "oidc-v1"
    hasher = hashlib.sha256()
    for name in ("manifest.json", "protocol-cases.json"):
        payload = load_json(corpus / name)
        canonical = json.dumps(payload, sort_keys=True, separators=(",", ":"))
        hasher.update(name.encode())
        hasher.update(b"\0")
        hasher.update(canonical.encode())
        hasher.update(b"\0")
    return hasher.hexdigest()


def validate_corpus(root: Path | None = None) -> list[str]:
    """Return a list of human-readable problems (empty means OK)."""
    root = root or repo_root_from()
    corpus = root / "conformance" / "oidc-v1"
    problems: list[str] = []
    try:
        manifest = load_json(corpus / "manifest.json")
        cases = load_json(corpus / "protocol-cases.json")
    except (OSError, json.JSONDecodeError) as exc:
        return [f"failed to load corpus: {exc}"]

    if manifest.get("provider_profile") != "1.0.0":
        problems.append(
            f"provider_profile must be 1.0.0, got {manifest.get('provider_profile')!r}"
        )
    if manifest.get("algorithm") != "EdDSA":
        problems.append(f"algorithm must be EdDSA, got {manifest.get('algorithm')!r}")
    if not manifest.get("security_profile"):
        problems.append("security_profile pointer missing")

    ids = {c.get("id") for c in manifest.get("cases", []) if isinstance(c, dict)}
    missing = sorted(REQUIRED_MANIFEST_IDS - ids)
    if missing:
        problems.append(f"manifest missing case ids: {', '.join(missing)}")

    redacted = manifest.get("redacted_fields") or []
    for field in (
        "access_token",
        "refresh_token",
        "id_token",
        "code",
        "code_verifier",
        "client_secret",
    ):
        if field not in redacted:
            problems.append(f"redacted_fields missing {field}")

    for family in ("authorization", "token", "refresh", "access_token", "userinfo", "metadata"):
        if family not in cases:
            problems.append(f"protocol-cases missing family {family}")

    return problems


def main() -> int:
    root = repo_root_from()
    problems = validate_corpus(root)
    digest = fixture_checksum(root)
    print(f"fixture_checksum={digest}")
    if problems:
        print("FAIL oidc conformance corpus:")
        for problem in problems:
            print(f"  - {problem}")
        return 1
    print("OK oidc conformance corpus")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
