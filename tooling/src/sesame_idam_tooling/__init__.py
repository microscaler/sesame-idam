"""Shim package: install ``brrtrouter-tooling`` and delegate to ``brrtrouter_tooling.workspace``."""

from importlib.metadata import PackageNotFoundError, version

try:
    __version__: str = version("sesame-idam-tooling")
except PackageNotFoundError:  # running from a source tree without the package installed
    __version__ = "0.0.0"

__all__: list[str] = ["__version__"]
