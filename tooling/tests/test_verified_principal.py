"""Epic 15.2 verified-principal mapping tests."""

from sesame_idam_tooling.verified_principal import (
    map_access_claims_to_principal,
    sample_access_claims,
    validate_principal,
)


def test_pre_org_maps_null_organization():
    principal = map_access_claims_to_principal(sample_access_claims(with_org=False))
    assert principal["organization_id"] is None
    assert principal["subject"] == "11111111-1111-1111-1111-111111111111"
    assert principal["session_id"] == "sid-1"
    assert principal["roles"] == ["owner"]
    assert validate_principal(principal) == []


def test_with_org_maps_organization_id():
    principal = map_access_claims_to_principal(sample_access_claims(with_org=True))
    assert principal["organization_id"] == "22222222-2222-2222-2222-222222222222"
    assert validate_principal(principal) == []
