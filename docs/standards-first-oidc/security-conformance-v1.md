# Sesame OIDC Security and Conformance v1

The normative protocol choices are in `provider-profile-v1.md`. This document
defines the release evidence required to claim compatibility.

## Fail-closed boundary

Any operation that mints a credential or creates authority must verify the
incoming credential before decoding claims. Verification includes EdDSA
signature, known active key, token type, exact issuer, intended audience, time,
tenant consistency, version, and denylist checks. Refresh state in Redis is
authoritative and bound to client and tenant.

Unknown algorithms, `alg=none`, unknown or duplicate keys, wrong key use/type,
critical headers, modified JWT segments, redirect prefix matching, PKCE plain,
code replay, refresh replay, and cross-client or cross-tenant substitution are
rejected.

## Shared corpus

`conformance/oidc-v1/manifest.json` is the language-neutral expected-outcome
index. `protocol-cases.json` contains transport vectors. Generated signed-token
vectors must record their deterministic keyset version and SHA-256 bundle
checksum; secrets and live production keys are forbidden.

Provider, resource-server, and client compatibility CI must consume the same
fixture version. A case cannot be skipped without a documented deviation,
owner, expiry date, and accepted ADR.

## Release gate

Release evidence records:

1. provider image digest and profile version;
2. discovery/JWKS route smoke results;
3. positive and negative fixture results;
4. external OpenID conformance-suite profile and report;
5. framework/library names and exact versions;
6. warm-cache key-rotation results;
7. log/trace/error redaction review;
8. known deviations and expiry dates.

The compatibility matrix includes Auth.js, Spring Security, ASP.NET Core,
Authlib, OmniAuth OpenID Connect, a maintained PHP OIDC implementation, and
`coreos/go-oidc`. Passing one parser is not sufficient.

No release may advertise a grant, response type, algorithm, endpoint, claim, or
authentication method that is absent from runtime and public routing.
