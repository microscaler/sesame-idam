# Postmortem: Loadlinker login outage — JWKS casing × stale deployment × phantom image tag

Date of incident: 2026-07-22 (latent since 2026-07-18)
Status: RESOLVED (fix-forward). Blameless.
Repos involved: sesame-idam (producer), hauliage/loadlinker (consumer),
BRRTRouter (validation library), shared-k8s registry + Flux image automation.
Also filed in: hauliage `docs/engineering/` (same document).

## Summary

After hauliage's BFF was rebuilt against BRRTRouter rev `70ea09d` (which
includes hardened, RFC-strict JWT/JWKS validation), every Sesame login was
rejected: *"Signed in with Sesame, but the BFF rejected the access token."*

The proximate trigger was the consumer's new strict JWKS parser. The actual
defect chain was three **latent, pre-existing failures** on the producer/
deployment side that the strict parser finally made visible:

1. The **deployed** `identity-session-service` was built from **2026-07-16
   code** and served JWKS with lowercase JOSE members (`"kty":"okp"`,
   `"crv":"ed25519"`) — a serde `rename_all` bug **already fixed** on sesame
   main on 2026-07-18 (`8f4573c`, guarded by `724f22e`).
2. The **post-fix image had vanished from the shared registry**; only the two
   pre-fix (Jul-16) tags remained pullable.
3. **Git pinned a phantom**: Flux image automation had committed tag
   `dev-1784457430340751330` (Jul-19) into
   `deployment-configuration/profiles/`, a tag that did not exist in the
   registry. The cluster kept running whatever it had last pulled.

No key material, token, or security boundary was compromised. Impact was
availability only: loadlinker dev-environment logins failed.

## Timeline (all 2026)

| When | What |
|---|---|
| Jul 16 | Last successful `identity-session-service` images pushed to the shared registry (pre-casing-fix). |
| Jul 18 10:51 | sesame `8f4573c` fixes JWKS serialization to exact RFC 8037 casing ("unblocks all token verification" — this bug had bitten before); `724f22e` adds a type-level guard so `rename_all` cannot regress it. **No rebuild/publish followed** — image publishing depends on a Tilt session being run. |
| Jul 19 | Flux image automation commits tag `dev-1784457430340751330` into the dev profile. That tag is (by Jul 22) absent from the registry — either never pushed durably or lost to registry churn. Nothing alerts on the pin↔registry mismatch. |
| Jul 20–22 | hauliage adopts octopilot; BRRTRouter pin advances `73744df` → `70ea09d`, crossing `d7444d2` "harden JWT validation": unconditional RFC 9068 `typ: at+jwt`, issuer algorithm allow-lists, and **strict RFC-cased JWKS parsing** (exact `OKP`/`Ed25519`/`EdDSA`, loud per-key rejection diagnostics). |
| Jul 22 | Rebuilt BFF loads the deployed (Jul-16) JWKS, strict parser rejects every key (`kty "okp"` miscased), zero keys cached → all logins rejected. |
| Jul 22 | Diagnosis: pin diff → validation.rs/typ/alg exonerated (sesame signs `at+jwt`/EdDSA; EdDSA in default allow-list; token-status defaults Active) → strict JWKS parser vs deployed casing → registry forensics reveal missing post-fix build and phantom git pin. |
| Jul 22 | Fix-forward: `identity-session-service` rebuilt from main (≥ `724f22e`) via the Tiltfile ritual replayed headlessly; published `dev-1784740485645080586`. Image automation re-converges git; pod rolls; logins restore after BFF JWKS cache refresh. |

## Root causes (layered — none sufficient alone)

1. **Producer serialization bug** (fixed pre-incident): serde `rename_all`
   lowercased case-sensitive JOSE members. JOSE member values are
   case-sensitive by RFC 7517/7518/8037; `okp` is not `OKP`.
2. **Publish-on-merge gap**: sesame image publishing is a side effect of a
   developer running Tilt. A merged fix does not become a deployable artifact
   until someone happens to build. The two-day fix→deploy gap was structural,
   not accidental.
3. **Registry/pin integrity gap**: nothing verifies that GitOps-pinned tags
   exist and are pullable; nothing retains or alerts on lost builds. Git
   described a deployment that could not exist.
4. **Consumer hardening rollout without a producer sweep**: the BRRTRouter pin
   bump crossed a security-hardening boundary; no conformance check of
   *deployed* token producers preceded the rollout.

## What went well

- **Strictness worked exactly as designed.** The consumer's parser rejected
  wrong casing with a precise, named diagnostic instead of tolerating it — a
  lenient parser would have hidden the producer bug indefinitely (and had:
  this same casing bug produced opaque 401s before `8f4573c`).
- The BFF's error message pointed at the JWKS dependency and service name.
- Forensics were fast and conclusive: pin diff → code exoneration by reading →
  registry API → git archaeology, ~an hour end to end.
- Fix-forward used the existing, versioned build ritual; no snowflake image.
- The (independently added) CI `paths-ignore` for
  `deployment-configuration/profiles/` prevented the fix's automation commit
  from triggering spurious builds.

## Decision affirmed

**RFC compliance remains enforced at consumers; producers are fixed at the
source.** Leniency migrates bugs from the component that owns them to every
component that consumes them, where they surface as undebuggable 401s.

## Action items

| # | Action | Repo/owner | Status |
|---|---|---|---|
| 1 | Onboard sesame-idam to octopilot so every merge to main produces published, pinned images (removes publish-on-merge gap) | sesame-idam | PLANNED — next adoption target |
| 2 | Registry integrity: alert on ImagePolicy/pin tags missing from the registry (Flux notification-controller) and on ImagePullBackOff for pinned refs; define retention for the shared registry | shared-gitops-k8s-cluster | OPEN |
| 3 | JWKS contract test at the boundary: consumer-grade conformance check (BRRTRouter's strict parser rules) against sesame's serialized JWKS in sesame CI — extend `724f22e`'s type-level guard with a parser-level round-trip | sesame-idam | OPEN |
| 4 | Pin-bump runbook: bumping BRRTRouter across security-hardening commits requires a deployed-producer conformance sweep first (curl JWKS, decode a token header) — add to hauliage/BRRTRouter upgrade checklist | hauliage | OPEN |
| 5 | Integration-deploy canary: hauliage's octopilot integration phase should include a login-path smoke against the deployed IdP once integration:true lands | hauliage | OPEN |

## Verification

```bash
curl -s http://identity-session-service.sesame-idam/.well-known/jwks.json \
  | jq '.keys[] | {kid, kty, crv, alg}'
# expect: "kty": "OKP", "crv": "Ed25519"
```

Then a fresh Sesame login through the BFF (after its JWKS cache TTL or a pod
restart).
