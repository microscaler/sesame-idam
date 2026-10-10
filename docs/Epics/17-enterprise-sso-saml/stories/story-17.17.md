# Story 17.17 — Deployment: verifier, job, secrets, policies

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** F. Delivery · **Depends on:** 17.5, 17.6 · **Size:** M  
> **Status:** Ready

## Goal

Everything the epic adds runs in a deployment (pp first) with least privilege.

## Context

[DESIGN-saml-sp.md §10](../../../DESIGN-saml-sp.md); [DESIGN-org-domain-verification.md §7](../../../DESIGN-org-domain-verification.md).

## Scope

- Chart values for `saml-verifier` (no service account token, read-only root, non-root, limits) and its NetworkPolicy (ingress from identity-login and org-mgmt only, egress none).
- The re-verification CronJob (17.5) with its own database login.
- Secrets: `K_enroll` and the SP key, generated in the secret backend, mounted only where used.
- Egress: org-mgmt and the job to the two DoH providers and HTTPS metadata URLs; identity-login unchanged.
- Configuration: `SESAME_SAML_VERIFIER_URL`, `SESAME_SAML_SP_KEY_FILE`, `SESAME_SAML_ENABLED`, DoH providers.

## Out of scope

- The deployment repository's own changes (tracked there).

## Design notes

- Defaults keep SAML disabled until a deployment enables it.

## Acceptance criteria

- [ ] A pod in another namespace, or a Sesame service other than the two, cannot reach the verifier; the verifier cannot reach anything.
- [ ] With `SESAME_SAML_ENABLED` unset no SAML route answers.

## Tests

- Chart render tests; policy tests in a kind cluster (connectivity matrix).

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.
