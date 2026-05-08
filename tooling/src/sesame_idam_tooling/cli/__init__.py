"""Shim: re-export `main` for the `sesame-idam` console script."""

from sesame_idam_tooling.cli.main import main

__all__ = ["main"]
