"""Platform tenant admin CLI — HTTP client for identity-login-service platform API."""

from __future__ import annotations

import argparse
import json
import os
import sys
import urllib.error
import urllib.request
from typing import Any

DEFAULT_LOGIN_URL = "http://127.0.0.1:8101/idam/v1"
PLATFORM_ADMIN_HEADER = "X-Platform-Admin-Key"


def login_service_url() -> str:
    return os.environ.get("SESAME_LOGIN_SERVICE_URL", DEFAULT_LOGIN_URL).rstrip("/")


def platform_admin_key() -> str:
    key = os.environ.get("SESAME_PLATFORM_ADMIN_KEY", "").strip()
    if not key:
        print(
            json.dumps(
                {
                    "error": "missing_config",
                    "error_description": "SESAME_PLATFORM_ADMIN_KEY is required",
                }
            ),
            file=sys.stderr,
        )
        sys.exit(1)
    return key


def _request(
    method: str,
    path: str,
    body: dict[str, Any] | None = None,
) -> tuple[int, dict[str, Any]]:
    url = f"{login_service_url()}{path}"
    data = None
    headers = {
        PLATFORM_ADMIN_HEADER: platform_admin_key(),
        "Accept": "application/json",
    }
    if body is not None:
        data = json.dumps(body).encode("utf-8")
        headers["Content-Type"] = "application/json"

    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            raw = resp.read().decode("utf-8")
            status = resp.status
    except urllib.error.HTTPError as exc:
        status = exc.code
        raw = exc.read().decode("utf-8")
    except urllib.error.URLError as exc:
        print(
            json.dumps(
                {
                    "error": "connection_error",
                    "error_description": str(exc.reason),
                }
            ),
            file=sys.stderr,
        )
        sys.exit(1)

    try:
        parsed = json.loads(raw) if raw else {}
    except json.JSONDecodeError:
        parsed = {"error": "invalid_json", "error_description": raw}
    return status, parsed


def _emit(status: int, body: dict[str, Any]) -> int:
    print(json.dumps(body, indent=2))
    return 0 if 200 <= status < 300 else 1


def cmd_create(args: argparse.Namespace) -> int:
    payload: dict[str, Any] = {
        "slug": args.slug,
        "display_name": args.display_name,
    }
    if args.no_activate:
        payload["activate"] = False
    status, body = _request("POST", "/platform/tenants", payload)
    return _emit(status, body)


def cmd_get(args: argparse.Namespace) -> int:
    status, body = _request("GET", f"/platform/tenants/{args.slug}")
    return _emit(status, body)


def cmd_status_set(args: argparse.Namespace) -> int:
    status, body = _request(
        "PATCH",
        f"/platform/tenants/{args.slug}/status",
        {"status": args.status},
    )
    return _emit(status, body)


def cmd_oauth_set(args: argparse.Namespace) -> int:
    payload: dict[str, Any] = {
        "client_id": args.client_id,
        "redirect_uris": [u.strip() for u in args.redirect_uris.split(",") if u.strip()],
        "secret_env_key": args.secret_env_key,
    }
    if args.client_id_env_key:
        payload["client_id_env_key"] = args.client_id_env_key
    status, body = _request(
        "PUT",
        f"/platform/tenants/{args.slug}/oauth/{args.provider}",
        payload,
    )
    return _emit(status, body)


def cmd_oauth_rotate(args: argparse.Namespace) -> int:
    status, body = _request(
        "POST",
        f"/platform/tenants/{args.slug}/oauth/{args.provider}/rotate",
        {"rotated_by": args.by},
    )
    return _emit(status, body)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="sesame-idam tenant")
    sub = parser.add_subparsers(dest="command", required=True)

    create = sub.add_parser("create", help="Mint a platform tenant")
    create.add_argument("--slug", required=True)
    create.add_argument("--display-name", required=True)
    create.add_argument("--no-activate", action="store_true")
    create.set_defaults(func=cmd_create)

    get = sub.add_parser("get", help="Get tenant detail + OAuth metadata")
    get.add_argument("--slug", required=True)
    get.set_defaults(func=cmd_get)

    status = sub.add_parser("status", help="Tenant lifecycle")
    status_sub = status.add_subparsers(dest="status_cmd", required=True)
    status_set = status_sub.add_parser("set", help="Set tenant status")
    status_set.add_argument("--slug", required=True)
    status_set.add_argument(
        "--status",
        required=True,
        choices=["active", "suspended", "deprovisioned", "provisioning", "failed"],
    )
    status_set.set_defaults(func=cmd_status_set)

    oauth = sub.add_parser("oauth", help="Tenant OAuth metadata")
    oauth_sub = oauth.add_subparsers(dest="oauth_cmd", required=True)

    oauth_set = oauth_sub.add_parser("set", help="Upsert OAuth provider metadata")
    oauth_set.add_argument("--slug", required=True)
    oauth_set.add_argument("--provider", required=True, choices=["google", "microsoft"])
    oauth_set.add_argument("--client-id", required=True)
    oauth_set.add_argument("--redirect-uris", required=True, help="Comma-separated URIs")
    oauth_set.add_argument("--secret-env-key", required=True)
    oauth_set.add_argument("--client-id-env-key")
    oauth_set.set_defaults(func=cmd_oauth_set)

    oauth_rotate = oauth_sub.add_parser("rotate", help="Record OAuth secret rotation")
    oauth_rotate.add_argument("--slug", required=True)
    oauth_rotate.add_argument("--provider", required=True)
    oauth_rotate.add_argument("--by", required=True, help="Actor email for audit")
    oauth_rotate.set_defaults(func=cmd_oauth_rotate)

    return parser


def run_tenant_cli(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args))


def main() -> int:
    return run_tenant_cli()
