# ADR-014: SAML signature verification — samael and libxmlsec1, in an isolated verifier

> **Status:** ACCEPTED (2026-10-10)
> **Design:** [DESIGN-saml-sp.md](DESIGN-saml-sp.md)

---

## Context

Sesame signs users in through their organisation's SAML identity provider (Entra ID, Okta and Google Workspace first).
Everything a SAML login asserts rests on one operation: verifying the XML signature over the IdP's response, and then
consuming exactly what was signed. That operation has a long record of failure across mature stacks:

- **Signature wrapping**: the signature verifies over one element while the application reads another (the
  XSW family; still being found: ruby-saml CVE-2024-45409, an authentication bypass rated CVSS 10.0).
- **Parser differentials**: the verifier and the consumer parse the same bytes into different trees (ruby-saml
  CVE-2025-25291 and CVE-2025-25292: REXML and Nokogiri parse one response into different trees, enabling wrapping
  with a single valid signature).
- **Comment and canonicalisation tricks**: a comment inside the NameID truncates it after signing (2018, several
  libraries at once).
- **Trusting the message's own key**: verifying against the certificate in `KeyInfo` instead of a pinned one.
- **XML parser attacks** on hostile input (external entities, entity expansion).

Sesame's services are pure Rust (ring, rustls), built by the Octopilot Rust buildpack. The Rust options in October
2026:

| Option | State |
| --- | --- |
| **samael** with `xmlsec` | SAML 2.0 SP/IdP library; signature verification through **libxmlsec1** (the reference XML-DSig implementation, with libxml2 and OpenSSL), with the IdP certificate set as the verification key, every `Signature` verified, an algorithm allow-list, and a reduction to only the verified content (default mode) against wrapping. Needs C libraries at build (libclang for bindgen, `-dev` packages) and run time. 0.0.22, July 2026. |
| **xml-sec** | Pure Rust XML-DSig and XML-Enc (C14N, exclusive C14N, RSA, ECDSA), a well-designed verification API (caller-trusted keys, expected signed targets, coverage proofs, fail-closed policy, two-parser differential mode). Seven months old, one author, agent-driven development, and it states it is not yet for production (design section 14). |
| Our own XML-DSig | Rejected outright. |

## Decision

### 1. samael with libxmlsec1 verifies SAML signatures

samael, `xmlsec` feature, **pinned to an exact version**, reviewed at each bump. Its verification is used with:

- the connection's pinned IdP certificates always configured (samael verifies nothing when it has none, so the wrapper
  refuses to build a verifier without at least one certificate);
- an explicit algorithm allow-list: RSA-SHA256/384/512, ECDSA-SHA256/384 (SHA-1, SHA-224 and DSA refused);
- the default reduction mode (only xmlsec-verified content survives; unsigned ancestors are stripped);
- IdP-initiated login disabled (`InResponseTo` must name the one pending request);
- Destination, Recipient, audience, issuer and time-window checks enabled, with at most 120 s clock skew.

### 2. It runs in its own process: `saml-verifier`

A seventh Sesame service, internal only, **stateless and secret-free**: no database, no Redis, no signing keys, no
egress, read-only file system, non-root, its own NetworkPolicy admitting only identity-login-service and org-mgmt. Hostile
XML is parsed by C libraries only in a process that holds nothing worth stealing and can reach nothing; a memory-safety
bug in libxml2 or libxmlsec1 is contained there instead of in the process that signs tokens and holds database
credentials.

identity-login-service and org-mgmt stay pure Rust and on the Rust buildpack. They call the verifier with everything a
decision needs (the response, the connection's pinned certificates, entity IDs, ACS URL, expected request ID, time) and
receive a verified, normalised result or a refusal.

### 3. Sesame enforces what samael does not

Before samael sees a response: size limit (256 KiB), strict base64, no DOCTYPE, no XML comments, no processing
instructions, no CDATA, no `EncryptedAssertion` (until encryption is designed in), exactly one `Assertion`. After: the assertion
ID equals the verified reference; `Status` is Success; an `AuthnStatement` is present; the assertion ID has not been seen
before (replay cache in identity-login, until `NotOnOrAfter`). The verifier refuses anything it cannot classify.

### 4. Build and supply chain

The verifier is built from a Dockerfile (Debian bookworm; build stage with the Rust toolchain, libclang and the
`-dev` packages; runtime stage with only libxmlsec1-openssl, libxml2, libxslt and OpenSSL), base images pinned by
digest, packages pinned by version, as a docker artifact of the same Octopilot pipeline. It is a separate image, so the
suite's run image is unchanged. A samael, libxmlsec1 or libxml2 bump is a pull request that must pass the full positive
and negative corpus (design section 9); security advisories for all three are watched.

### 5. AuthnRequests are signed without XML signing

SP-initiated login uses the HTTP-Redirect binding, where the signature covers the query string, not XML: identity-login
signs it (RSA-SHA256) with the deployment's SP key, which lives in Cloud KMS at HSM protection level
(`RSA_SIGN_PKCS1_3072_SHA256`, ADR-015) and signs through `asymmetricSign`; a local key file serves development. No XML
is signed by Sesame.

### 6. xml-sec is a side project, not on this path

libxmlsec1 is the verifier, with no shadow or consensus stage gating delivery. xml-sec (audit in the design, section 14)
asks the right questions, and a pure-Rust verifier would remove the C code, but bringing it to production readiness is
work we cannot put in front of SAML. It is a separate, later effort: help its author get it production ready (fixtures,
an audit of the modules a SAML verifier uses, differential fuzzing against libxmlsec1, a minimal feature profile) and
possibly give it a production user. Adopting it, in shadow, in consensus or as a replacement, is a new ADR; the
verifier's API does not change if its internals do.

### 4a. Amendment (2026-10-11): standard service, own workspace, own image

`saml-verifier` has the standard Sesame service shape (OpenAPI `openapi/idam/saml-verifier/openapi.yaml`, BRRTRouter
gen and impl crates, `RunAppBuilder`, health and metrics, the shared chart) but lives in a Cargo workspace of its own,
`microservices/saml-verifier` (`core`, `gen`, `impl`), excluded from the suite workspace and built by
`docker/saml-verifier/Dockerfile`, which runs the tests in its build stage. Linking samael's `xmlsec` feature into a
suite binary would have put libxmlsec1, libxml2 and libclang into the buildpack builder, the suite's run image and the
CI runner. Keeping the C code behind `core` and in one image also makes it removable: a pure-Rust verifier (xml-sec,
§6) replaces `core` and the Dockerfile, not the service or its contract. Base images are pinned by digest; Debian
packages come from that release and are recorded in the image (`/app/packages.txt`) instead of exact apt pins, which
Debian security updates would remove from the mirrors.

## Consequences

- Seven services; the new one is small, has a narrow internal API and no state. (Domain verification later adds an
  eighth, `dns-probe`, the mirror image: egress to two fixed DNS-over-HTTPS providers and nothing else; ADR-007
  amendment of 2026-10-11.)
- C code (libxml2, libxmlsec1, OpenSSL) enters Sesame, confined to one process with no secrets and no network reach.
- Two XML parsers still meet (libxml2 verifies, samael deserialises with quick-xml), but only over libxml2's own
  serialisation of the verified subtree, after the pre-checks of section 3, and with the assertion ID cross-checked; the
  negative corpus includes parser-differential cases.
- Upgrading samael is deliberate work, not a routine Dependabot merge.
- xml-sec is not adopted now; working on it is a separate side project (section 6), and adopting it a new ADR.

## Alternatives rejected

- **xml-sec (pure Rust)**, as verifier, in shadow or in consensus: not production ready by its own statement and
  unaudited; making it so would delay SAML. A side project instead (section 6).
- **samael linked into identity-login-service**: C parsing of hostile input in the process holding signing keys and
  database credentials; a run image with C libraries for the whole suite.
- **An external SAML broker** (a second IdP such as Keycloak or Dex in front of Sesame): another identity system to
  operate and secure, duplicating Sesame's own job.
- **A verifier in another language** (e.g. Go and its SAML libraries): those libraries have their own wrapping
  advisories, and a second toolchain buys nothing over libxmlsec1.
