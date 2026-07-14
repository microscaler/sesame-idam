"""Unit tests for platform tenant CLI HTTP client."""

from __future__ import annotations

import io
import json
from unittest import mock

import pytest

from sesame_idam_tooling.cli import tenant as tenant_cli


def test_create_posts_tenant_payload(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("SESAME_PLATFORM_ADMIN_KEY", "test-key")
    monkeypatch.setenv("SESAME_LOGIN_SERVICE_URL", "http://example.test/idam/v1")

    captured: dict[str, object] = {}

    def fake_request(method, path, body=None):
        captured["method"] = method
        captured["path"] = path
        captured["body"] = body
        return 201, {"slug": "acme", "status": "active"}

    monkeypatch.setattr(tenant_cli, "_request", fake_request)

    rc = tenant_cli.run_tenant_cli(
        ["create", "--slug", "acme", "--display-name", "Acme Corp"]
    )
    assert rc == 0
    assert captured == {
        "method": "POST",
        "path": "/platform/tenants",
        "body": {"slug": "acme", "display_name": "Acme Corp"},
    }


def test_missing_platform_key_exits(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("SESAME_PLATFORM_ADMIN_KEY", raising=False)
    stderr = io.StringIO()
    with mock.patch("sys.stderr", stderr):
        with pytest.raises(SystemExit) as exc:
            tenant_cli.run_tenant_cli(["get", "--slug", "acme"])
    assert exc.value.code == 1
    payload = json.loads(stderr.getvalue())
    assert payload["error"] == "missing_config"


def test_api_error_returns_nonzero(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("SESAME_PLATFORM_ADMIN_KEY", "test-key")

    monkeypatch.setattr(
        tenant_cli,
        "_request",
        lambda *_args, **_kwargs: (409, {"error": "slug_taken"}),
    )

    rc = tenant_cli.run_tenant_cli(
        ["create", "--slug", "taken", "--display-name", "Taken"]
    )
    assert rc == 1
