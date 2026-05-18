"""Sesame-IDAM CLI shim.

Monkey-patches brrtrouter_tooling workspace discovery to use sesame-idam's
nested layout: openapi/idam/<service>/ and microservices/idam/<service>/.
"""

import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Monkey-patch brrtrouter-tooling discovery layers BEFORE main() runs
# ---------------------------------------------------------------------------

import brrtrouter_tooling.workspace.discovery.suites as _h_suites

_original_suites_with_bff = _h_suites.suites_with_bff


def patched_suites_with_bff(project_root: Path) -> list[str]:
    result = _original_suites_with_bff(project_root)
    d = project_root / "openapi" / "idam"
    if d.exists() and (d / "bff-suite-config.yaml").exists():
        if "idam" not in result:
            result.append("idam")
    return result


_h_suites.suites_with_bff = patched_suites_with_bff

_original_bff_path = _h_suites.bff_suite_config_path


def patched_bff_suite_config_path(project_root: Path, suite: str) -> Path:
    if suite == "idam":
        return project_root / "openapi" / "idam" / "bff-suite-config.yaml"
    return _original_bff_path(project_root, suite)


_h_suites.bff_suite_config_path = patched_bff_suite_config_path

_original_openapi_bff = _h_suites.openapi_bff_path


def patched_openapi_bff_path(project_root: Path, suite: str) -> Path:
    if suite == "idam":
        return project_root / "openapi" / "idam" / "bff-suite-config.yaml"
    return _original_openapi_bff(project_root, suite)


_h_suites.openapi_bff_path = patched_openapi_bff_path

_original_service_to_suite = _h_suites.service_to_suite


def patched_service_to_suite(project_root: Path, service_name: str) -> str | None:
    d = project_root / "openapi" / "idam"
    if d.exists() and (d / service_name / "openapi.yaml").exists():
        return "idam"
    return _original_service_to_suite(project_root, service_name)


_h_suites.service_to_suite = patched_service_to_suite

_original_suite_sub = _h_suites.suite_sub_service_names


def patched_suite_sub_service_names(project_root: Path, suite: str) -> list[str]:
    if suite == "idam":
        d = project_root / "openapi" / "idam"
        if not d.exists() or not d.is_dir():
            return []
        return sorted(
            x.name for x in d.iterdir()
            if x.is_dir() and (x / "openapi.yaml").exists()
        )
    return _original_suite_sub(project_root, suite)


_h_suites.suite_sub_service_names = patched_suite_sub_service_names

_original_iter_suite = _h_suites.iter_suite_services


def patched_iter_suite_services(project_root: Path, suite: str | None = None):
    if suite is None or suite == "idam":
        d = project_root / "openapi" / "idam"
        if d.exists() and d.is_dir():
            for name in sorted(
                x.name for x in d.iterdir()
                if x.is_dir() and (x / "openapi.yaml").exists()
            ):
                yield ("idam", name)
    yield from _original_iter_suite(project_root, suite)


_h_suites.iter_suite_services = patched_iter_suite_services

_original_tilt = _h_suites.tilt_service_names


def patched_tilt_service_names(project_root: Path) -> list[str]:
    result = set(_original_tilt(project_root))
    d = project_root / "openapi" / "idam"
    if d.exists() and d.is_dir():
        for name in d.iterdir():
            if name.is_dir() and (name / "openapi.yaml").exists():
                result.add(name.name)
    return sorted(result)


_h_suites.tilt_service_names = patched_tilt_service_names

_original_load = _h_suites.load_suite_services


def patched_load_suite_services(project_root: Path) -> set:
    result = _original_load(project_root)
    d = project_root / "openapi" / "idam"
    if d.exists() and d.is_dir():
        for name in d.iterdir():
            if name.is_dir() and (name / "openapi.yaml").exists():
                result.add(name.name)
    return result


_h_suites.load_suite_services = patched_load_suite_services

# ---------------------------------------------------------------------------
# Monkey-patch discovery.services for build tool (build calls get_package_names
# which imports iter_suite_services directly, so patches on _h_suites don't propagate)
# Read actual package/binary names from Cargo.toml instead of guessing.
# ---------------------------------------------------------------------------

import brrtrouter_tooling.workspace.discovery.services as _h_services


def _read_impl_package_name(project_root: Path, service_name: str) -> str:
    """Read impl/Cargo.toml [package].name for a service."""
    impl_cargo = project_root / "microservices" / "idam" / service_name / "impl" / "Cargo.toml"
    if not impl_cargo.exists():
        return ""
    try:
        import configparser
        cfg = configparser.ConfigParser()
        cfg.read(str(impl_cargo))
        val = cfg.get("package", "name", fallback="")
        return val.strip().strip('"')
    except Exception:
        return ""


def _read_impl_binary_name(project_root: Path, service_name: str) -> str:
    """Read impl/Cargo.toml [[bin]] name for a service."""
    impl_cargo = project_root / "microservices" / "idam" / service_name / "impl" / "Cargo.toml"
    if not impl_cargo.exists():
        return ""
    try:
        with open(impl_cargo) as f:
            content = f.read()
        import re
        m = re.search(r'\[\[bin\]\]\s*name\s*=\s*"([^"]+)"', content)
        if m:
            return m.group(1)
    except Exception:
        pass
    return ""


_original_get_package_names = _h_services.get_package_names


def patched_get_package_names(project_root: Path, suite: str | None = None) -> dict[str, str]:
    """Cargo [package].name per service with idam suite support.

    Reads impl/Cargo.toml for the actual package name (e.g. sesame_idam_authz_core_gen_impl).
    """
    out: dict[str, str] = {}
    for _s, service_name in _h_suites.iter_suite_services(project_root, suite=suite):
        pkg_name = _read_impl_package_name(project_root, service_name)
        if pkg_name:
            out[service_name] = pkg_name
    for bff_svc, _s in _h_suites.iter_bffs(project_root, suite=suite):
        pkg_name = _read_impl_package_name(project_root, bff_svc)
        if pkg_name:
            out[bff_svc] = pkg_name
    return out


_original_get_binary_names = _h_services.get_binary_names


def patched_get_binary_names(project_root: Path, suite: str | None = None) -> dict[str, str]:
    """Binary name per service with idam suite support.

    Reads [[bin]] name from impl/Cargo.toml.
    """
    out: dict[str, str] = {}
    for _s, service_name in _h_suites.iter_suite_services(project_root, suite=suite):
        bin_name = _read_impl_binary_name(project_root, service_name)
        if bin_name:
            out[service_name] = bin_name
        else:
            out[service_name] = service_name.replace("-", "_")
    for bff_svc, _ in _h_suites.iter_bffs(project_root, suite=suite):
        bin_name = _read_impl_binary_name(project_root, bff_svc)
        if bin_name:
            out[bff_svc] = bin_name
        else:
            out[bff_svc] = bff_svc.replace("-", "_")
    return out


_original_get_service_ports = _h_services.get_service_ports


def patched_get_service_ports(project_root: Path) -> dict[str, str]:
    """HTTP port per service with idam suite support."""
    out: dict[str, str] = {}
    for name, (_suite, port) in _h_services.discover_openapi_suite_microservice_localhost(project_root).items():
        out[name] = str(port)
    for name, port in _h_services.discover_bff_suite_config(project_root).items():
        out.setdefault(name, str(port))
    for name, port in _h_services.discover_helm(project_root).items():
        out.setdefault(name, str(port))
    return out


_h_services.get_package_names = patched_get_package_names
_h_services.get_binary_names = patched_get_binary_names
_h_services.get_service_ports = patched_get_service_ports

# ---------------------------------------------------------------------------
# Monkey-patch regenerate.py to use sesame-idam's nested paths
# ---------------------------------------------------------------------------

import brrtrouter_tooling.workspace.gen.regenerate as _regenerate


def patched_regenerate_service(
    project_root: Path,
    suite: str,
    service_name: str,
    brrtrouter_path: Path | None = None,
) -> int:
    """Regenerate a sesame-idam service with nested openapi/idam/ paths."""
    spec_path = project_root / "openapi" / "idam" / service_name / "openapi.yaml"
    deps_config_path = spec_path.parent / "brrtrouter-dependencies.toml"
    output_dir = project_root / "microservices" / "idam" / service_name / "gen"

    if not spec_path.exists():
        print(f"❌ OpenAPI spec not found: {spec_path}")
        return 1

    # Sesame-IDAM uses a custom naming convention: sesame_idam_{service}_gen
    # (not the default BRRTRouter {snake}_service_api).
    # Replace dashes with underscores so Cargo accepts the package name
    # (Cargo forbids dashes in package identifiers).
    safe_name = service_name.replace("-", "_")
    package_name = f"sesame_idam_{safe_name}_gen"

    if brrtrouter_path is None:
        brrtrouter_path = _regenerate.discover_brrtrouter_root(project_root)

    is_bff = _regenerate.bff_service_to_suite(project_root, service_name) == suite

    try:
        result = _regenerate.call_brrtrouter_generate(
            spec_path=spec_path,
            output_dir=output_dir,
            project_root=project_root,
            brrtrouter_path=brrtrouter_path,
            deps_config_path=deps_config_path if deps_config_path.exists() else None,
            package_name=package_name,
            capture_output=False,
        )

        if result.returncode != 0:
            print(f"❌ Failed to regenerate {service_name}")
            return 1

        print(f"✅ Regenerated {service_name}")

        gen_cargo = output_dir / "Cargo.toml"
        if gen_cargo.exists():
            _regenerate._fix_cargo_paths_callback(gen_cargo, project_root)

        return 0
    except FileNotFoundError as e:
        print(f"❌ {e}")
        return 1


_regenerate.regenerate_service = patched_regenerate_service

# ---------------------------------------------------------------------------
# Delegate to brrtrouter_tooling workspace CLI
# ---------------------------------------------------------------------------

from brrtrouter_tooling.workspace.cli.main import main

__all__ = ["main"]
