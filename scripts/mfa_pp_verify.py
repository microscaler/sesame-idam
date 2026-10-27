#!/usr/bin/env python3
"""ADR-008 phase A verification against Sesame in pp, with throwaway test users only.

Covers: password sign-in claims; security session; TOTP enrolment, second step, replay refusal, lockout-free wrong code;
recovery code; passkey registration and username-less sign-in with a software authenticator (ES256, "none"
attestation, user verification, backup-eligible); organisation policy (mfa -> forced enrolment; passkey -> passkey only);
factor removal refused by policy. Prints PASS/FAIL per check; never prints tokens, secrets or codes.

Usage: TEST_EMAIL_BASE='someone+{tag}@your-domain' mfa_pp_verify.py <client_id>

Sesame in pp sends real mail (sign-in codes, security notices) through the production relay, so every test user
needs a deliverable address. TEST_EMAIL_BASE is a plus-addressed template with a `{tag}` placeholder, filled with a
unique tag per user (e.g. someone+mfa-verify-20261027-a@your-domain). The script refuses to run without it, and
refuses reserved, non-deliverable domains (example.com/.net/.org, .test, .invalid, .localhost, .example), which can
only bounce.
"""
import base64, hashlib, hmac, json, os, struct, sys, time, urllib.request, urllib.error

from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.asymmetric import ec

HOST = "https://sesame-idam.pp.pricewhisperer.ai"
TENANT = "pricewhisperer"
CLIENT = sys.argv[1]
RP_ID = "pp.pricewhisperer.ai"
ORIGIN = HOST
RUN = time.strftime("%Y%m%d%H%M%S")
EMAIL_BASE = os.environ.get("TEST_EMAIL_BASE", "").strip()
RESERVED_DOMAINS = ("example.com", "example.net", "example.org")
RESERVED_TLDS = ("test", "invalid", "localhost", "example")


def reserved(domain):
    d = domain.lower().rstrip(".")
    return d.rsplit(".", 1)[-1] in RESERVED_TLDS or any(d == r or d.endswith("." + r) for r in RESERVED_DOMAINS)


def test_email(tag):
    """The address for one test user: TEST_EMAIL_BASE with {tag} filled in."""
    return EMAIL_BASE.replace("{tag}", tag)


if "{tag}" not in EMAIL_BASE or "@" not in EMAIL_BASE:
    sys.exit("TEST_EMAIL_BASE must be a deliverable address template containing {tag}, e.g. someone+{tag}@your-domain")
if reserved(EMAIL_BASE.rsplit("@", 1)[1]):
    sys.exit("TEST_EMAIL_BASE is at a reserved, non-deliverable domain: mail to it can only bounce")
results = []


def check(name, ok, detail=""):
    results.append((name, bool(ok)))
    print(("PASS " if ok else "FAIL ") + name + (f"  [{detail}]" if detail and not ok else ""), flush=True)


def call(path, body=None, bearer=None, method="POST"):
    data = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(HOST + path, data=data, method=method)
    req.add_header("Content-Type", "application/json")
    req.add_header("X-Tenant-ID", TENANT)
    if bearer:
        req.add_header("Authorization", "Bearer " + bearer)
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            raw = r.read()
            return r.status, (json.loads(raw) if raw else {})
    except urllib.error.HTTPError as e:
        raw = e.read()
        try:
            return e.code, json.loads(raw)
        except Exception:
            return e.code, {"raw": raw[:200].decode(errors="replace")}


def claims(jwt):
    p = jwt.split(".")[1]
    return json.loads(base64.urlsafe_b64decode(p + "=" * (-len(p) % 4)))


def b64u(b):
    return base64.urlsafe_b64encode(b).rstrip(b"=").decode()


def unb64u(s):
    return base64.urlsafe_b64decode(s + "=" * (-len(s) % 4))


def totp(secret_b32, t=None, step_offset=0):
    key = base64.b32decode(secret_b32 + "=" * (-len(secret_b32) % 8))
    counter = int((t or time.time()) // 30) + step_offset
    d = hmac.new(key, struct.pack(">Q", counter), hashlib.sha1).digest()
    o = d[-1] & 15
    return "%06d" % ((struct.unpack(">I", d[o:o + 4])[0] & 0x7FFFFFFF) % 1000000)


# ── minimal CBOR (maps, ints, bytes, text) ─────────────────────────────────────────────────────────────────────────
def cbor(v):
    def head(major, n):
        if n < 24:
            return bytes([major << 5 | n])
        if n < 256:
            return bytes([major << 5 | 24, n])
        if n < 65536:
            return bytes([major << 5 | 25]) + struct.pack(">H", n)
        return bytes([major << 5 | 26]) + struct.pack(">I", n)
    if isinstance(v, bool):
        return bytes([0xF5 if v else 0xF4])
    if isinstance(v, int):
        return head(0, v) if v >= 0 else head(1, -1 - v)
    if isinstance(v, bytes):
        return head(2, len(v)) + v
    if isinstance(v, str):
        b = v.encode()
        return head(3, len(b)) + b
    if isinstance(v, dict):
        return head(5, len(v)) + b"".join(cbor(k) + cbor(x) for k, x in v.items())
    raise TypeError(v)


class SoftAuthenticator:
    """A platform-style authenticator: ES256, user verification, backup-eligible ("synced") credentials."""

    def __init__(self):
        self.creds = {}  # cred_id -> (key, user_handle, counter)

    def create(self, options):
        pk = options["publicKey"]
        assert pk["rp"]["id"] == RP_ID, pk["rp"]
        key = ec.generate_private_key(ec.SECP256R1())
        nums = key.public_key().public_numbers()
        cred_id = os.urandom(32)
        cose = cbor({1: 2, 3: -7, -1: 1, -2: nums.x.to_bytes(32, "big"), -3: nums.y.to_bytes(32, "big")})
        flags = 0x01 | 0x04 | 0x08 | 0x10 | 0x40  # UP UV BE BS AT
        auth_data = hashlib.sha256(RP_ID.encode()).digest() + bytes([flags]) + struct.pack(">I", 0) \
            + b"\0" * 16 + struct.pack(">H", len(cred_id)) + cred_id + cose
        client_data = json.dumps({"type": "webauthn.create", "challenge": pk["challenge"], "origin": ORIGIN,
                                  "crossOrigin": False}).encode()
        self.creds[cred_id] = [key, unb64u(pk["user"]["id"]), 0]
        return {"id": b64u(cred_id), "rawId": b64u(cred_id), "type": "public-key",
                "response": {"attestationObject": b64u(cbor({"fmt": "none", "attStmt": {}, "authData": auth_data})),
                             "clientDataJSON": b64u(client_data), "transports": ["internal"]},
                "extensions": {"credProps": {"rk": True}}}

    def get(self, options, cred_id=None):
        pk = options["publicKey"]
        assert pk.get("rpId") == RP_ID, pk.get("rpId")
        allowed = [unb64u(c["id"]) for c in pk.get("allowCredentials") or []]
        cid = cred_id or next(c for c in self.creds if not allowed or c in allowed)
        key, handle, counter = self.creds[cid]
        counter += 1
        self.creds[cid][2] = counter
        auth_data = hashlib.sha256(RP_ID.encode()).digest() + bytes([0x01 | 0x04 | 0x08 | 0x10]) + struct.pack(">I", counter)
        client_data = json.dumps({"type": "webauthn.get", "challenge": pk["challenge"], "origin": ORIGIN,
                                  "crossOrigin": False}).encode()
        sig = key.sign(auth_data + hashlib.sha256(client_data).digest(), ec.ECDSA(hashes.SHA256()))
        return {"id": b64u(cid), "rawId": b64u(cid), "type": "public-key",
                "response": {"authenticatorData": b64u(auth_data), "clientDataJSON": b64u(client_data),
                             "signature": b64u(sig), "userHandle": b64u(handle)},
                "extensions": {}}


def register(tag):
    email = test_email(f"mfa-verify-{RUN}-{tag}")
    pw = "Verify-" + b64u(os.urandom(12))
    s, b = call("/idam/v1/auth/register", {"client_id": CLIENT, "email": email, "password": pw,
                                            "first_name": "MFA", "last_name": "Verify " + tag})
    check(f"register test user {tag}", s in (200, 201) and "access_token" in b, f"{s} {b.get('error')}")
    return email, pw


def login(email, pw):
    return call("/idam/v1/auth/login", {"client_id": CLIENT, "email": email, "password": pw})


def manage(access):
    s, b = call("/idam/v1/auth/mfa/manage", {}, bearer=access)
    check("security session opens for a fresh sign-in", s == 200 and b.get("mfa_token"), f"{s} {b.get('error')}")
    return b.get("mfa_token")


# ── user A: password, TOTP, recovery, passkey ─────────────────────────────────────────────────────────────────────
email, pw = register("a")
s, b = login(email, pw)
check("password sign-in without a factor: 200", s == 200, f"{s} {b.get('error')}")
c = claims(b["access_token"])
check("token amr is [pwd], acr aal1, auth_time set",
      c.get("amr") == ["pwd"] and c.get("acr") == "urn:sesame:acr:aal1" and abs(c.get("auth_time", 0) - time.time()) < 120,
      f"{c.get('amr')} {c.get('acr')}")
mt = manage(b["access_token"])

s, f = call("/idam/v1/auth/mfa/factors", {"mfa_token": mt})
check("factors: none yet; passkeys and TOTP available", s == 200 and f["factors"] == [] and f["passkeys_available"]
      and f["totp_available"], f"{s} {f}")

s, e = call("/idam/v1/auth/mfa/totp/enroll", {"mfa_token": mt, "label": "Verify app"})
check("TOTP enrol returns a secret and otpauth URI", s == 200 and e.get("otpauth_uri", "").startswith("otpauth://totp/"),
      f"{s} {e.get('error')}")
s, x = call("/idam/v1/auth/mfa/totp/confirm", {"mfa_token": mt, "factor_id": e["factor_id"], "code": "000000"})
check("TOTP confirm refuses a wrong code", s == 401 and x.get("error") == "invalid_code", f"{s} {x.get('error')}")
s, x = call("/idam/v1/auth/mfa/totp/confirm", {"mfa_token": mt, "factor_id": e["factor_id"], "code": totp(e["secret"])})
codes = x.get("recovery_codes") or []
check("TOTP confirmed; first factor returns 10 recovery codes", s == 200 and len(codes) == 10, f"{s} {x.get('error')}")

s, b = login(email, pw)
check("password sign-in now answers 202 (no token)", s == 202 and "access_token" not in b and b.get("mfa_token"),
      f"{s}")
check("challenge offers totp and recovery_code", sorted(b.get("methods", [])) == ["recovery_code", "totp"], b.get("methods"))
tok = b["mfa_token"]
s, x = call("/idam/v1/auth/mfa/verify", {"mfa_token": tok, "method": "totp", "code": "123456"})
check("wrong TOTP refused", s == 401, f"{s} {x.get('error')}")
time.sleep(31)  # the enrolment used the current step; a sign-in code must be a later one
code = totp(e["secret"])
s, x = call("/idam/v1/auth/mfa/verify", {"mfa_token": tok, "method": "totp", "code": code})
ok = s == 200 and "access_token" in x
check("correct TOTP signs in", ok, f"{s} {x.get('error')}")
if ok:
    c = claims(x["access_token"])
    check("amr [mfa, otp, pwd], acr aal2", c.get("amr") == ["mfa", "otp", "pwd"] and c.get("acr") == "urn:sesame:acr:aal2",
          f"{c.get('amr')} {c.get('acr')}")
    access_mfa = x["access_token"]
s, b = login(email, pw)
s, x = call("/idam/v1/auth/mfa/verify", {"mfa_token": b["mfa_token"], "method": "totp", "code": code})
check("the same TOTP code is refused the second time (replay)", s == 401, f"{s}")

s, b = login(email, pw)
s, x = call("/idam/v1/auth/mfa/verify", {"mfa_token": b["mfa_token"], "method": "recovery_code", "code": codes[0]})
ok = s == 200 and "access_token" in x
check("recovery code signs in", ok, f"{s} {x.get('error')}")
if ok:
    c = claims(x["access_token"])
    check("recovery: amr [mfa, pwd, rc], acr aal2 (not phishing-resistant)",
          c.get("amr") == ["mfa", "pwd", "rc"] and c.get("acr") == "urn:sesame:acr:aal2", f"{c.get('amr')}")
s, b = login(email, pw)
s, x = call("/idam/v1/auth/mfa/verify", {"mfa_token": b["mfa_token"], "method": "recovery_code", "code": codes[0]})
check("a used recovery code is refused", s == 401, f"{s}")

# Passkey registration in a security session (needs an MFA'd fresh sign-in now).
s, b = login(email, pw)
s, x = call("/idam/v1/auth/mfa/verify", {"mfa_token": b["mfa_token"], "method": "recovery_code", "code": codes[1]})
mt = manage(x["access_token"])
auth = SoftAuthenticator()
s, o = call("/idam/v1/auth/passkeys/register/options", {"mfa_token": mt})
check("passkey creation options: RP ID pp.pricewhisperer.ai, UV required, discoverable preferred",
      s == 200 and o["options"]["publicKey"]["rp"]["id"] == RP_ID
      and o["options"]["publicKey"]["authenticatorSelection"]["userVerification"] == "required"
      and o["options"]["publicKey"]["authenticatorSelection"]["residentKey"] == "preferred", f"{s} {o.get('error')}")
s, r = call("/idam/v1/auth/passkeys/register", {"mfa_token": mt, "challenge_id": o["challenge_id"],
                                                "credential": auth.create(o["options"]), "label": "Verify passkey"})
check("passkey registered (synced)", s == 200 and r.get("factor", {}).get("kind") == "passkey"
      and r["factor"].get("synced") is True and not r.get("recovery_codes"), f"{s} {r.get('error')}")
s, o2 = call("/idam/v1/auth/passkeys/register/options", {"mfa_token": mt})
s, r2 = call("/idam/v1/auth/passkeys/register", {"mfa_token": mt, "challenge_id": o["challenge_id"],
                                                 "credential": auth.create(o["options"])})
check("a WebAuthn challenge is answered once", s == 401, f"{s}")

s, o = call("/idam/v1/auth/passkeys/challenge", {"client_id": CLIENT})
check("username-less options (conditional mediation)", s == 200 and o["options"].get("mediation") == "conditional",
      f"{s} {o.get('error')}")
s, x = call("/idam/v1/auth/passkeys/verify", {"challenge_id": o["challenge_id"], "credential": auth.get(o["options"])})
ok = s == 200 and "access_token" in x
check("username-less passkey sign-in: 200 without a password", ok, f"{s} {x.get('error')}")
if ok:
    c = claims(x["access_token"])
    check("passkey: amr [mfa, swk, user], acr aal2-phishing-resistant",
          c.get("amr") == ["mfa", "swk", "user"] and c.get("acr") == "urn:sesame:acr:aal2-phishing-resistant",
          f"{c.get('amr')} {c.get('acr')}")
    passkey_access = x["access_token"]

s, b = login(email, pw)
check("challenge now also offers passkey", "passkey" in b.get("methods", []), b.get("methods"))
s, o = call("/idam/v1/auth/passkeys/challenge", {"mfa_token": b["mfa_token"]})
s, x = call("/idam/v1/auth/passkeys/verify", {"challenge_id": o["challenge_id"], "credential": auth.get(o["options"]),
                                              "mfa_token": b["mfa_token"]})
ok = s == 200 and "access_token" in x
check("passkey as the second factor signs in", ok, f"{s} {x.get('error')}")
if ok:
    c = claims(x["access_token"])
    check("pwd + passkey: amr [mfa, pwd, swk, user]", c.get("amr") == ["mfa", "pwd", "swk", "user"], f"{c.get('amr')}")

# Organisation policy: A creates an org and requires passkeys.
s, org = call("/idam/v1/organizations", {"name": f"MFA verify {RUN}"}, bearer=passkey_access)
check("test organisation created", s in (200, 201) and org.get("id"), f"{s} {org}")
s, p = call(f"/idam/v1/organizations/{org['id']}/auth-policy", {"require": "passkey"}, bearer=passkey_access, method="PUT")
check("owner sets policy passkey", s == 200 and p.get("require") == "passkey", f"{s} {p}")
s, p = call(f"/idam/v1/organizations/{org['id']}/auth-policy", None, bearer=passkey_access, method="GET")
check("policy reads back", s == 200 and p.get("require") == "passkey", f"{s}")
s, b = login(email, pw)
check("under a passkey policy only passkey (+ recovery) is offered", s == 202 and b.get("methods") == ["passkey", "recovery_code"],
      b.get("methods"))
s, x = call("/idam/v1/auth/mfa/verify", {"mfa_token": b["mfa_token"], "method": "totp", "code": totp(e["secret"])})
check("TOTP refused under a passkey policy", s == 400, f"{s}")
s, x = call("/idam/v1/auth/mfa/verify", {"mfa_token": b["mfa_token"], "method": "recovery_code", "code": codes[2]})
check("recovery under a passkey policy leads to passkey enrolment, not tokens",
      s == 202 and x.get("enrollment_required") == "passkey" and "access_token" not in x, f"{s} {x}")
s, o = call("/idam/v1/auth/passkeys/challenge", {"client_id": CLIENT})
s, x = call("/idam/v1/auth/passkeys/verify", {"challenge_id": o["challenge_id"], "credential": auth.get(o["options"])})
mt = manage(x["access_token"])
s, f = call("/idam/v1/auth/mfa/factors", {"mfa_token": mt})
pk = next(z for z in f["factors"] if z["kind"] == "passkey")
s, x = call("/idam/v1/auth/mfa/factors/remove", {"mfa_token": mt, "factor_id": pk["id"]})
check("the last passkey cannot be removed under a passkey policy", s == 409 and x.get("error") == "policy_requires_factor",
      f"{s} {x.get('error')}")
app = next(z for z in f["factors"] if z["kind"] == "totp")
s, x = call("/idam/v1/auth/mfa/factors/remove", {"mfa_token": mt, "factor_id": app["id"]})
check("the authenticator app can be removed", s == 200 and all(z["kind"] != "totp" for z in x["factors"]), f"{s}")
call(f"/idam/v1/organizations/{org['id']}/auth-policy", {"require": "none"}, bearer=passkey_access, method="PUT")

# ── user B: no factor, organisation requires MFA -> forced enrolment ──────────────────────────────────────────────
email_b, pw_b = register("b")
s, b = login(email_b, pw_b)
s, orgb = call("/idam/v1/organizations", {"name": f"MFA verify B {RUN}"}, bearer=b["access_token"])
s, p = call(f"/idam/v1/organizations/{orgb['id']}/auth-policy", {"require": "mfa"}, bearer=b["access_token"], method="PUT")
check("user B sets policy mfa on their organisation", s == 200, f"{s}")
s, b = login(email_b, pw_b)
check("member without a factor must enrol (202 enrollment_required mfa)",
      s == 202 and b.get("enrollment_required") == "mfa" and b.get("methods") == [], f"{s} {b}")
s, e = call("/idam/v1/auth/mfa/totp/enroll", {"mfa_token": b["mfa_token"]})
s, x = call("/idam/v1/auth/mfa/totp/confirm", {"mfa_token": b["mfa_token"], "factor_id": e["factor_id"], "code": totp(e["secret"])})
ok = s == 200 and x.get("tokens", {}).get("access_token")
check("enrolment completes the sign-in (tokens + recovery codes)", ok and len(x.get("recovery_codes") or []) == 10,
      f"{s} {x.get('error')}")
if ok:
    c = claims(x["tokens"]["access_token"])
    check("the enrolment sign-in is reported truthfully (amr [pwd], aal1)", c.get("amr") == ["pwd"]
          and c.get("acr") == "urn:sesame:acr:aal1", f"{c.get('amr')}")
s, b = login(email_b, pw_b)
check("next sign-in requires the new factor", s == 202 and b.get("methods") == ["totp", "recovery_code"], f"{s} {b.get('methods')}")

failed = [n for n, ok in results if not ok]
print(f"\n{len(results) - len(failed)}/{len(results)} checks passed")
sys.exit(1 if failed else 0)
