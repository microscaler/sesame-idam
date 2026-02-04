"""`sesame` CLI entry point. Same guard rails as RERP; tilt subcommands for DX (same as RERP)."""

import argparse
import sys
from pathlib import Path

from sesame_idam_tooling.tilt.setup_kind_registry import run as run_setup_kind_registry
from sesame_idam_tooling.tilt.setup_persistent_volumes import (
    run as run_setup_persistent_volumes,
)


def _project_root() -> Path:
    return Path.cwd()


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="sesame",
        description="Sesame-IDAM development tooling: tilt (same DX as RERP)",
    )
    sub = parser.add_subparsers(dest="command", help="Commands")
    pt = sub.add_parser("tilt", help="Tilt/dev: setup-kind-registry, setup-persistent-volumes")
    pt_sub = pt.add_subparsers(dest="tilt_cmd")
    pt_sub.add_parser("setup-kind-registry", help="Create/start local registry, connect to kind")
    pt_sub.add_parser(
        "setup-persistent-volumes",
        help="Apply k8s/data and k8s/monitoring PersistentVolumes if present",
    )
    return parser.parse_args()


def main() -> None:
    """Dispatch to tilt subcommands or print help."""
    args = _parse_args()
    root = _project_root()
    if args.command == "tilt":
        t = getattr(args, "tilt_cmd", None)
        if t == "setup-kind-registry":
            sys.exit(run_setup_kind_registry(root))
        if t == "setup-persistent-volumes":
            sys.exit(run_setup_persistent_volumes(root))
        print("sesame tilt: missing subcommand", file=sys.stderr)
        print("  setup-kind-registry, setup-persistent-volumes", file=sys.stderr)
        print("  Use: sesame tilt --help", file=sys.stderr)
        sys.exit(1)
    if args.command is None:
        print("Sesame-IDAM tooling: use justfile from repo root.", file=sys.stderr)
        print("  just init          # Create tooling/.venv and install [dev]", file=sys.stderr)
        print("  just dev-up        # Kind + registry + Tilt (same DX as RERP)", file=sys.stderr)
        print("  just qa            # Lint + format-check + pytest", file=sys.stderr)
        print("  sesame tilt --help # Tilt subcommands", file=sys.stderr)
        sys.exit(0)
    sys.exit(1)
