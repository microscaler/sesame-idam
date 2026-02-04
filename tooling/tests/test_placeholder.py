"""Placeholder test so QA (pytest) passes. Add tests as tooling grows."""

import pytest

from sesame_idam_tooling import __version__


def test_version_is_string() -> None:
    """Package has a version string."""
    assert isinstance(__version__, str)
    assert len(__version__) >= 5  # e.g. 0.1.0
