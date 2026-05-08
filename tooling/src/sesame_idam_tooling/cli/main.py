"""Thin shim: `sesame-idam` CLI delegates to brrtrouter_tooling.workspace."""

from brrtrouter_tooling.workspace.cli.main import main

__all__ = ["main"]
