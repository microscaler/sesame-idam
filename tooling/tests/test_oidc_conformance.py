"""Tests for the OIDC conformance fixture gate."""

from sesame_idam_tooling.oidc_conformance import fixture_checksum, validate_corpus
from sesame_idam_tooling.oidc_framework_matrix import (
    FRAMEWORK_SLICE,
    validate_access_negatives,
)


def test_corpus_validates_clean():
    assert validate_corpus() == []


def test_fixture_checksum_stable_hex():
    digest = fixture_checksum()
    assert len(digest) == 64
    assert all(c in "0123456789abcdef" for c in digest)
    assert fixture_checksum() == digest


def test_framework_slice_pins_present():
    assert "authjs" in FRAMEWORK_SLICE
    assert "authlib" in FRAMEWORK_SLICE
    assert "spring_resource_server" in FRAMEWORK_SLICE
    assert validate_access_negatives() == []
