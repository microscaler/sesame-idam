import yaml

with open("/home/casibbald/Workspace/microscaler/seasame-idam/openapi/idam/identity-login-service/openapi.yaml") as f:
    data = yaml.safe_load(f)

tr = data["components"]["schemas"]["TokenResponse"]
print("TokenResponse properties:", list(tr.get("properties", {}).keys()))
print("TokenResponse required:", tr.get("required", []))
print()
print("entitlements_ref:", tr["properties"].get("entitlements_ref"))
print()
print("entitlements_hash:", tr["properties"].get("entitlements_hash"))
print()
print("roles:", tr["properties"].get("roles"))
print()
print("permissions:", tr["properties"].get("permissions"))

# Check /users/me endpoint
print()
if "/auth/users/me" in data["paths"]:
    print("/auth/users/me endpoint EXISTS")
    op = data["paths"]["/auth/users/me"]["get"]
    print("operationId:", op.get("operationId"))
    print("tags:", op.get("tags"))
else:
    print("/auth/users/me endpoint MISSING")

print()
print("UserProfileResponse exists:", "UserProfileResponse" in data["components"]["schemas"])
print("EntitlementsSnapshot exists:", "EntitlementsSnapshot" in data["components"]["schemas"])

# Check that email/email_verified/phone_verified are NOT in TokenResponse props
props = tr.get("properties", {})
assert "email" not in props, "FAIL: email still in TokenResponse"
assert "email_verified" not in props, "FAIL: email_verified still in TokenResponse"
assert "phone_verified" not in props, "FAIL: phone_verified still in TokenResponse"
print("\nAll PII fields removed from TokenResponse: OK")

# Check SocialLoginResponse and SocialCallbackResponse were cleaned
social_login = data["components"]["schemas"]["SocialLoginResponse"]
sl_props = social_login.get("properties", {})
assert "email" not in sl_props, "FAIL: email still in SocialLoginResponse"
assert "email_verified" not in sl_props, "FAIL: email_verified still in SocialLoginResponse"
print("SocialLoginResponse PII removed: OK")
