# Client ecosystem selection v1

## Selected first integration

Auth.js is selected as the first **Reference** integration, not a supported SDK.
It covers a large TypeScript web ecosystem while preserving Auth.js as the
maintained OIDC implementation. The reference lives under
`ui/reference-authjs` and contains only issuer/client defaults and profile
mapping.

Owner: Sesame platform team  
Provider profile: `1.0.x`  
Public API: `1.0.x`  
Maintenance: exact-version compatibility CI; security reports follow the
repository security policy.

## Deferred candidates

ASP.NET Core, Spring, Python/Authlib, Laravel/PHP, Rails/OmniAuth, Go, and
framework-neutral Rust remain unselected until customer demand and a named
maintainer justify a release-blocking support commitment. Generic libraries in
those ecosystems are conformance consumers in Epic 14; that does not make them
Sesame SDKs.

## Package rule

Any future package must remain thin over the ecosystem OIDC/JWT implementation,
consume provider discovery, map only cryptographically verified principals,
support optional organization state, use the shared fixture version, publish a
supported-version matrix and provenance, and never add a provider workaround.
Provider incompatibilities are fixed in Sesame.
