# Compatibility and deprecation policy v1

Status: normative  
Profile version: `1.0.0`

## Version identifiers

| Identifier | Meaning |
|---|---|
| `provider_profile` | Normative OIDC + claim profile (semver) |
| `fixture_version` | Conformance corpus revision under `conformance/oidc-v1` |
| Tenant-consumer `info.version` | Public OpenAPI document version |
| Client package version | SDK release; must declare supported profile/API |

`conformance/oidc-v1/VERSION` records `provider_profile` and `fixture_version`
together. Checksums are produced by
`python -m sesame_idam_tooling.oidc_conformance`.

## Breaking vs non-breaking

**Breaking (major `provider_profile` or OpenAPI major)**

- removing or renaming a required claim or principal field;
- changing algorithm, `typ`, or required validation rules;
- removing a public path/operation from tenant-consumer OpenAPI;
- changing error codes that clients branch on;
- making a previously optional claim required without a migration window.

**Non-breaking (minor / patch)**

- additive optional claims or OpenAPI fields (consumers MUST ignore unknowns);
- new fixture negative cases that do not change accept semantics of existing ids;
- documentation clarifications.

## Required lockstep on profile bumps

When `provider_profile` increments:

1. Update claim tables in `provider-profile-v1.md` (or publish `v2`).
2. Regenerate fixture checksum; update `VERSION` and `CHECKSUM`.
3. Publish an OpenAPI diff note (even if OpenAPI is unchanged).
4. Bump supported versions in `sesame-idam-client` and proof consumers.

## Deprecation

Deprecated behavior requires:

1. a migration note under `docs/standards-first-oidc/` or epic evidence;
2. a published removal window (minimum one minor profile cycle);
3. fixture coverage that still accepts the deprecated shape until removal.

## Client release rule

Every client release states:

- supported `provider_profile` range;
- supported tenant-consumer OpenAPI version;
- supported `fixture_version` (exact or range).
