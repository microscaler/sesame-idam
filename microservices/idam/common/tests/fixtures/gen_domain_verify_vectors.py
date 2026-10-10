#!/usr/bin/env python3
"""Generate domain_verify_vectors.json: an independent implementation of the token construction
(DESIGN-org-domain-verification.md section 3, DESIGN-tenant-domain-verification.md section 2).

The keys below are public test keys, not secrets. Run: python3 gen_domain_verify_vectors.py > domain_verify_vectors.json
"""
import base64
import hashlib
import hmac
import json

KEYS = {
    "k32_00_1f": bytes(range(32)),
    "k32_aa": b"\xaa" * 32,
    "k48_ascii": b"sesame-domain-verify test key, never production!",
}


def token(key: bytes, fields: list[str]) -> str:
    mac = hmac.new(key, "\x00".join(fields).encode("utf-8"), hashlib.sha256).digest()
    return base64.urlsafe_b64encode(mac).rstrip(b"=").decode("ascii")


CASES = [
    # (key, kind, tenant, org, input, normalised, scope)
    ("k32_00_1f", "organisation", "acme", "b2000002-0002-4000-8000-000000000002", "bigbank.com", "bigbank.com", "exact"),
    ("k32_00_1f", "organisation", "acme", "b2000002-0002-4000-8000-000000000002", "bigbank.com", "bigbank.com", "subdomains"),
    ("k32_00_1f", "organisation", "acme", "b2000002-0002-4000-8000-000000000003", "bigbank.com", "bigbank.com", "exact"),
    ("k32_00_1f", "organisation", "globex", "b2000002-0002-4000-8000-000000000002", "bigbank.com", "bigbank.com", "exact"),
    ("k32_aa", "organisation", "acme", "b2000002-0002-4000-8000-000000000002", "bigbank.com", "bigbank.com", "exact"),
    ("k32_aa", "organisation", "acme", "b2000002-0002-4000-8000-000000000002", "EU.BigBank.co.uk.", "eu.bigbank.co.uk", "exact"),
    ("k48_ascii", "organisation", "pricewhisperer", "0193a4b2-7c1e-7d3a-9f00-1234567890ab", "BüCher.de", "xn--bcher-kva.de", "subdomains"),
    ("k32_00_1f", "tenant_issuer", "acme", None, "idam.acme.com", "idam.acme.com", None),
    ("k32_00_1f", "tenant_issuer", "acme", None, "acme.com", "acme.com", None),
    ("k48_ascii", "tenant_issuer", "loadlinker", None, "Idam.LoadLinker.com.", "idam.loadlinker.com", None),
]

vectors = []
for key_id, kind, tenant, org, raw, fqdn, scope in CASES:
    if kind == "organisation":
        fields = [tenant, org, fqdn, scope, "sesame-org-domain", "v1"]
    else:
        fields = [tenant, fqdn, "sesame-domain-verify", "v1"]
    t = token(KEYS[key_id], fields)
    vectors.append({
        "key_id": key_id,
        "kind": kind,
        "tenant_id": tenant,
        "org_id": org,
        "scope": scope,
        "input": raw,
        "domain": fqdn,
        "record_name": f"_sesame-challenge.{fqdn}",
        "record_value": f"sesame-domain-verify=v1;t={t}",
        "token": t,
    })

print(json.dumps({
    "description": "Sesame domain verification token vectors (v1). Keys are public test values.",
    "keys": {k: base64.urlsafe_b64encode(v).rstrip(b"=").decode("ascii") for k, v in KEYS.items()},
    "vectors": vectors,
}, indent=2, ensure_ascii=False))
