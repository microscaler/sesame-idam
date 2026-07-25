# Stock-take, 2026-07-25 — disjointed positions

Written after finding two features shipped in violation of an unstated rule
(ADR-011), and then finding the rule itself was only half-applied. This is an
audit of every place where **the code, the ADRs and the deployed reality
disagree**, so each one is either fixed or has a task and an owner instead of
living in someone's memory.

Ordered by exposure, not by effort.

---

## 1. OPEN — OAuth/SSO config is platform-key-only (ADR-011 violation, live)

`/platform/tenants/{slug}/oauth/{provider}` and `.../rotate` are
`PlatformServiceAuth`. A tenant wiring up its own Google or Microsoft sign-in
would need the platform operator key, which ADR-011 §1 says can never be issued
to them.

Identical in shape to the SMS flaw, which is now fixed — this is the same bug
left standing. Nothing can be self-served today, so no tenant has been given a
platform key; the exposure is that the feature is unusable as designed rather
than that it is being misused.

**Fix:** `/tenant/oauth/{provider}` GET/PUT/POST-rotate, bearer + tenant-admin,
tenant from the token, delegating to the same service layer as the platform
variant (as the SMS pair now does).

→ **Task 33** — tenant-scoped OAuth/SSO

---

## 2. OPEN — the authority boundary is untested (ADR-011 §4)

ADR-011 names four obligations. None are tested:

1. A tenant token cannot reach another tenant's data.
2. A tenant token is rejected on `/platform/*`.
3. A platform key is rejected on `/tenant/*`.
4. A tenant user without the admin role is refused on `/tenant/*`.

3 and 4 have unit coverage in `services/tenant_admin.rs`; none has an
end-to-end test. 2 and 3 matter most — they are the cases that would silently
start passing if someone made one credential accept the other, which is
exactly the kind of "helpful" change that looks like a simplification.

**Fix:** BDD covering all four from both directions.

→ **Task 30** — boundary BDD

---

## 3. OPEN — security schemes fail open in BRRTRouter

A declared-but-unconfigured `apiKey` scheme falls back to the literal key
`test123` and the service starts normally.

Not currently exploitable in Sesame: the affected scheme (`ApiKeyHeader`)
appears only in the document-level `security:` list, and the router does not
implement OpenAPI's top-level OR semantics, so presenting `test123` is
rejected. **Both of those are accidents.** Making the router spec-compliant on
OR — a reasonable correctness fix — would turn every operation inheriting that
list into one accepting `test123` instead of a JWT.

Full analysis and proposed uplift:
[`BRRTRouter/docs/DESIGN-security-scheme-fail-closed.md`](../../BRRTRouter/docs/DESIGN-security-scheme-fail-closed.md).

→ **Task 34** (router uplift), **Task 35** (Sesame specs)

---

## 4. OPEN — document-level `security` declares an alternative nobody means

All six service specs carry:

```yaml
security:
- BearerAuth: []
- ApiKeyHeader: []
```

No operation intends `X-API-KEY` as an alternative to a JWT. The specs should
say what they mean, so their safety does not depend on router behaviour.

→ **Task 35**

---

## 5. OPEN — JWKS is not externally reachable, so its rate limit is unproven

Gate A1 budgets `/idam/v1/.well-known/jwks.json`, but JWKS is served by
identity-session-service and no frontend proxies it, so the policy has never
been exercised. Either expose it deliberately (normal for an OIDC surface,
along with `openid-configuration`) and re-test, or drop the group and record
that JWKS is internal-only.

→ **Task 36** — JWKS exposure decision

---

## 6. OPEN — tenant console screens do not exist for the new endpoints

`/tenant/sms/{environment}` is implemented; the console screen still needs
repointing, and there is no SSO screen. Building these is how the last two
authority bugs were found, so it is investigation as much as delivery.

→ **Task 31** — console screens

---

## 7. OPEN — self-service tenant registration

Brochure CTA → signup → email possession → tenant provisioned → first
`tenant_admin` → console. Spec was drafted and deliberately reverted rather
than left with no controllers and stale codegen.

Design decided: public signup creates a **lesser** thing than platform
provisioning (`provisioning` status, which the tenant gate refuses for
authentication), the slug is derived server-side so there is no "name taken"
enumeration oracle, and activation requires email possession.

→ **Task 32** — registration journey

---

## 8. RECORDED — ADR-007 domain verification is the Tier 1 mechanism

Fully specified, unbuilt. It now carries more weight than when written: it is
the free rung of the assurance ladder (ADR-011 §5) and the answer to granting
the first `tenant_admin` on self-service signup. Not a disjoint position — a
dependency that several other things now rest on.

→ **Task 37** — ADR-007 domain verification

---

## 9. CLOSED — resolved during this stock-take

- **SMS config platform-key-only** — fixed; `/tenant/sms/{environment}` ships,
  both surfaces share one service layer.
- **Multi-provider SMS** — decided: Twilio Connect only. Adding a provider
  would mean routinely holding other companies' credentials, which is a
  different product. ADR-009 records the decision, its commercial edge and the
  revisit cost.
- **KYB cost** — decided: assurance ladder with `KycProvider::Disabled` by
  default. Every capability is reachable at `domain_verified`, asserted by a
  test, so the flag being off is not an outage.
- **`git add -A` emptied ten tracked files** — fixed in `af14912`. Cause was
  staging during an NFS-visible rewrite window. Rule: explicit paths only in
  the ms02 checkouts.

---

## Standing lesson

Every item above shares one shape: **something silently did nothing, or
silently allowed something, where it should have refused loudly.** The NULL
that vanished, the redirect that never fired, the rate limit on paths that did
not exist, the security scheme defaulting to a known key, the platform key
required from someone who could never hold one.

The cheapest defence has consistently been asking *who concretely does this,
and what happens if the answer is nobody* — which is why building the console
found two authority bugs that tests did not.
