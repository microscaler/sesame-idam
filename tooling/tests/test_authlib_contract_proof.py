import pytest

from sesame_idam_tooling.authlib_contract_proof import (
    prove_principal_from_accepted_claims,
    validate_discovery_fixture,
)


def test_discovery_fixtures_present():
    assert validate_discovery_fixture() == []


def test_principal_proof_without_authlib_import_path():
    # Mapping/schema proof does not require Authlib; CLI entry does.
    principal = prove_principal_from_accepted_claims()
    assert principal["profile_version"] == "1.0.0"
    assert principal["organization_id"] is None


def test_main_requires_authlib_or_passes(monkeypatch):
    import sesame_idam_tooling.authlib_contract_proof as mod

    class FakeAuthlib:
        __version__ = "1.3.2"

    monkeypatch.setitem(__import__("sys").modules, "authlib", FakeAuthlib())
    assert mod.main() == 0
