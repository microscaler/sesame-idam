from sesame_idam_tooling.contract_sync import validate_contract_sync


def test_contract_versions_lockstep():
    assert validate_contract_sync() == []
