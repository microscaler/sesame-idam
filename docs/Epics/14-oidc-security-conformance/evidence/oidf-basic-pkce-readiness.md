# OIDF Basic / Authorization Code + PKCE — readiness report

**Date:** 2026-08-03  
**Profile:** OpenID Foundation Conformance Suite — Basic OP (Authorization Code + PKCE)  
**Issuer:** `https://id.sesameidentity.dev.local`  
**Client:** `fixture-public-client`  
**provider_profile:** `1.0.0` (see `conformance/oidc-v1/manifest.json`)

## Status

**Not certified.** This document records readiness evidence and the first live
surface probe. Full OIDF suite execution is release-blocking for
`provider_profile` bumps (Epic 14.6 / 14.9) but is not claimed green here.

## Live surface probe (2026-08-03)

| Check | Expected | Result |
|-------|----------|--------|
| `GET {issuer}/.well-known/openid-configuration` | 200, `issuer` match | See `oidf-discovery-probe.json` |
| `authorization_endpoint` | Hosted auth / authorize | Advertised |
| `token_endpoint` | `api.` host | Advertised |
| `userinfo_endpoint` | `api.` host | Advertised |
| `jwks_uri` | Reachable EdDSA keys | Advertised |
| `response_types_supported` | `["code"]` only | Contract-tested in `oidc_live_api` |
| `code_challenge_methods_supported` | `["S256"]` | Contract-tested |
| Interactive PKCE round-trip | authorize→login→complete→token→userinfo | Green (`live_interactive_pkce_round_trip`) |

## Suite run instructions

1. Register / use `fixture-public-client` with redirect(s) accepted by OIDF.
2. Point OIDF Basic + PKCE plan at `https://id.sesameidentity.dev.local`.
3. Export HTML/JSON report under this `evidence/` directory.
4. Fix provider gaps; do not bump `provider_profile` until green.

## Gaps to close before certification claim

- Persist full OIDF HTML/JSON artifact from an automated runner
- Confirm logout / session management profile scope (out of Basic slice)
- Align any advertise≠implement leftovers discovered by the suite
