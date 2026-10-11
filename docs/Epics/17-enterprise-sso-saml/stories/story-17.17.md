# Story 17.17 — Deployment: verifier, probe, job, secrets, policies

> **Epic:** [17 Enterprise SSO](../README.md) · **Track:** F. Delivery · **Depends on:** 17.5, 17.6 · **Size:** M  
> **Status:** Ready

## Goal

Everything the epic adds runs in a deployment (pp first) with least privilege.

## Context

[DESIGN-saml-sp.md §10](../../../DESIGN-saml-sp.md); [DESIGN-org-domain-verification.md §7](../../../DESIGN-org-domain-verification.md).

## Scope

- Chart values for `saml-verifier` (no service account token, read-only root, non-root, limits) and its NetworkPolicy (ingress from identity-login and org-mgmt only, egress none).
- Chart values for `dns-probe` (same hardening) and its NetworkPolicy (ingress from org-mgmt and the re-verification job only; egress only to the configured DoH providers' addresses on 443).
- The re-verification CronJob (17.5) with its own database login.
- `K_enroll`: an HMAC-SHA256 key in Cloud KMS at HSM protection level (created by the platform, automatic rotation off); `roles/cloudkms.signer` on that key only for the org-mgmt and re-verification job workload identities; egress for both to the metadata server and Cloud KMS; `SESAME_DOMAIN_VERIFY_KEYRING` names key and versions.
- Secrets: the SP key, generated in the secret backend, mounted only where used.
- Egress: org-mgmt to public addresses on 443 only (IdP metadata), excluding private, loopback, link-local, CGNAT and cluster ranges; the job none beyond `dns-probe` and the database; identity-login unchanged.
- Configuration: `SESAME_SAML_VERIFIER_URL`, `SESAME_SAML_SP_KEY_FILE`, `SESAME_SAML_ENABLED`, `SESAME_DNS_PROBE_URL`; DoH providers in `dns-probe`'s own configuration.

## Out of scope

- The deployment repository's own changes (tracked there).

## Design notes

- Defaults keep SAML disabled until a deployment enables it.

## Acceptance criteria

- [ ] A pod in another namespace, or a Sesame service other than the two, cannot reach the verifier; the verifier cannot reach anything.
- [ ] Only org-mgmt and the job reach `dns-probe`; `dns-probe` reaches only its two providers; org-mgmt cannot reach the cloud metadata server or any cluster-internal address through its egress.
- [ ] With `SESAME_SAML_ENABLED` unset no SAML route answers.
- [ ] Only org-mgmt and the job can use the `K_enroll` key, the key version is HSM-protected, and no pod mounts it.

## Tests

- Chart render tests; policy tests in a kind cluster (connectivity matrix).

## Definition of done

- Acceptance criteria met with tests in CI; rustfmt and clippy clean; OpenAPI and docs updated where touched;
  security-relevant behaviour covered by a negative test; evidence noted in the epic README.
