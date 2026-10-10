# DESIGN: SAML in Sesame

Status: design, accepted for implementation (October 2026). Owner: Charles Sibbald.
Decision record: [ADR-014](ADR-014-saml-signature-verification.md) (samael and libxmlsec1, in an isolated verifier).

Sesame signs users in through their organisation's SAML 2.0 identity provider. First IdPs: **Microsoft Entra ID, Okta,
Google Workspace**. First flow: **SP-initiated**, HTTP-Redirect binding out, HTTP-POST binding back. IdP-initiated login,
single logout, encrypted assertions and SCIM come later (section 13).

SAML is an upstream login method, like Google or Microsoft social login: the IdP vouches for a person, Sesame decides
who that is in which tenant and organisation, and Sesame alone issues tokens. A relying party never sees a SAML message
and needs no SAML code.

---

## 1. Today

| Piece | State |
| --- | --- |
| `saml_connections` table (org-mgmt) | `org_id`, `issuer`, `metadata_url`, `sso_url`, `signing_cert` |
| org-mgmt SAML operations (allow, enable, disable, link, metadata, delete) | stubs that audit and echo; paths not organisation-scoped (`/sso/saml/…`) |
| `sesame-idam-client` | `saml_login_start`, `saml_callback`, `saml_domain_discovery` aligned with section 4 (tenant from `client_id`, code bound to client and redirect URI); pre-contract, with a pending contract test that gates once identity-login publishes the operations |
| Contract tests (`pact-mock-server/tests/saml_track_a_contract.rs`) | describe the target paths; ignored, run in the advisory CI job, and become gating when this lands |
| SAML parsing or verification | none |
| Social login (Google, Microsoft) | complete; the model SAML follows (state in Redis, shared `issue_login_response`) |

## 2. Architecture

```
 browser ──► RP / hosted auth UI
                │ GET /auth/saml/login                      ┌──────────────────────────┐
                ▼                                           │ saml-verifier (internal) │
 identity-login-service (public routes) ── POST /verify ──► │ samael + libxmlsec1      │
   state, replay cache: Redis                               │ stateless, no secrets,   │
   SP key: secret backend (signs AuthnRequest, ring)        │ no DB, no Redis, no      │
   connections, identities: Postgres (RLS)                  │ egress                   │
                ▲                                           └──────────────────────────┘
 org-mgmt (admin API) ── POST /metadata/parse ──────────────────────▲
   connection lifecycle, metadata fetch (egress, HTTPS only)
```

- **identity-login-service** owns the sign-in flow: starts it (AuthnRequest), receives the IdP's POST at the assertion
  consumer service (ACS), asks the verifier, links the identity, completes the login.
- **saml-verifier** (new, ADR-014) owns XML: verifies responses and parses IdP metadata. Internal only, stateless,
  holds nothing worth stealing, reaches nothing. libxmlsec1 (through samael) verifies.
- **org-mgmt** owns connections: who may configure SAML, the IdP metadata, enabling and disabling.

identity-login-service and org-mgmt stay pure Rust; only the verifier links C libraries.

## 3. Sign-in flow (SP-initiated)

```mermaid
sequenceDiagram
    participant B as Browser
    participant RP as Relying party
    participant L as identity-login
    participant V as saml-verifier
    participant I as Organisation IdP
    B->>RP: Sign in with SSO
    RP->>L: GET /auth/saml/login?org_id&client_id&redirect_uri
    L->>L: connection enabled? redirect_uri registered for client_id?
    L->>L: AuthnRequest(ID), state {request ID, tenant, org, connection, client, redirect_uri} in Redis 10 min
    L-->>B: 302 IdP SSO URL ?SAMLRequest&RelayState&SigAlg&Signature
    B->>I: authenticate
    I-->>B: auto-submitting form
    B->>L: POST /auth/saml/acs/{connection_id} SAMLResponse, RelayState
    L->>L: consume state (one use); state.connection == path connection
    L->>V: POST /verify {response, certs, entity IDs, ACS, request ID, now}
    V-->>L: verified {name_id, attributes, assertion ID, session_not_on_or_after} | refused
    L->>L: replay check; resolve or provision user (section 6)
    L->>L: saml_access_code in Redis (60 s, one use, bound to client_id + redirect_uri)
    L-->>B: 302 redirect_uri?code=…
    B->>RP: follows
    RP->>L: POST /auth/saml/callback {saml_access_code, client_id[, client_secret]}
    L-->>RP: TokenResponse (as every login)
```

- **Hosted auth UI.** "Sign in with SSO" takes an email address, finds the organisation whose **verified** domain
  (ADR-007) it is, and starts the flow above with the pending OIDC authorize transaction as its `redirect_uri`
  (`/oauth/authorize/complete`), so an OIDC relying party receives an ordinary authorization code. Domain discovery
  answers the same way whether or not a domain has SSO (no enumeration).
- **RelayState** is an opaque random key; everything it stands for is server side.
- **Failures** after the IdP returns redirect the browser to the RP's `redirect_uri` with `error=access_denied` (no
  detail) when the state is valid, and to a static error page otherwise; the cause is in the audit log.

## 4. Endpoints

### identity-login-service

| operationId | Method, path | Access | Notes |
| --- | --- | --- | --- |
| `saml_login` | `GET /auth/saml/login` | public | `org_id`, `client_id`, `redirect_uri`; 302 to the IdP |
| `saml_acs` | `POST /auth/saml/acs/{connection_id}` | public | `application/x-www-form-urlencoded`: `SAMLResponse`, `RelayState` |
| `saml_callback` | `POST /auth/saml/callback` | public (RP back end) | code exchange; client authentication as for the token endpoint |
| `saml_sp_metadata` | `GET /auth/saml/sp/{connection_id}/metadata` | public | SP metadata XML (built with quick-xml; no signing) |
| `saml_domain_discovery` | `POST /auth/saml/discover` | public, rate-limited | `{email, client_id}` → `{org_id}` or nothing; tenant from the client |

One SP entity per connection: entity ID `https://<public host>/idam/v1/auth/saml/sp/{connection_id}`, ACS
`https://<public host>/idam/v1/auth/saml/acs/{connection_id}`. Each organisation's IdP application is then independent,
an audience names exactly one connection, and a connection can be deleted without touching another.

### org-mgmt (organisation-scoped)

| operationId | Method, path | Who |
| --- | --- | --- |
| `get_saml` | `GET /organizations/{org_id}/sso/saml` | org admin |
| `delete_saml` | `DELETE /organizations/{org_id}/sso/saml` | org admin |
| `allow_org_saml` | `POST /organizations/{org_id}/sso/saml/allow` | tenant admin: the org may use SAML (entitlement) |
| `disallow_org_saml` | `POST /organizations/{org_id}/sso/saml/disable` | tenant admin: withdraws it; the connection stops working |
| `enable_saml` | `POST /organizations/{org_id}/sso/saml/enable` | org admin, only after a successful test sign-in |
| `create_saml_link` | `POST /organizations/{org_id}/sso/saml/link` | org admin: a one-time setup link (7 days) for the org's IT admin |
| `set_saml_idp_metadata` | `PUT /organizations/{org_id}/sso/saml/metadata` | org admin or link holder: metadata XML or an HTTPS metadata URL |

The unscoped `/sso/saml/*` paths are removed (the contract test forbids them). org-mgmt derives the organisation from the
path and the validated token, never from a header (FINDING-2026-07-25).

### saml-verifier (internal)

| Method, path | Request | Response |
| --- | --- | --- |
| `POST /verify` | `saml_response` (base64), `idp_entity_id`, `idp_certs` (DER, base64), `sp_entity_id`, `acs_url`, `request_id`, `now`, `max_clock_skew_secs` | `200 {name_id, name_id_format, attributes: {name: [values]}, assertion_id, not_on_or_after, session_not_on_or_after, authn_instant}` or `422 {reason}` (a fixed code, never echoing input) |
| `POST /metadata/parse` | `metadata` (XML, ≤ 1 MiB) | `200 {entity_id, sso_redirect_url, signing_certs: [{der, not_after}], name_id_formats}` or `422 {reason}` |

## 5. Verification rules

All mandatory. The verifier refuses anything it cannot classify; identity-login treats every refusal the same way.

**Before samael (verifier):**
- `SAMLResponse` ≤ 256 KiB, strict base64, UTF-8.
- No DOCTYPE, no XML comments, no processing instructions, no CDATA, no `EncryptedAssertion` (scanned with quick-xml on the
  raw document; comments matter because a comment inside a NameID truncates it after signing).
- Exactly one `Assertion` element anywhere in the document.
- At least one pinned certificate (samael verifies nothing without one).

**samael with libxmlsec1** (configured as ADR-014 §1):
- every `Signature` verifies with a pinned certificate; `KeyInfo` never supplies a key;
- algorithms RSA-SHA256/384/512, ECDSA-SHA256/384 only;
- default reduction: only verified content survives (signature-wrapping defence);
- `Destination` = ACS when present or when the Response is signed; issuer = `idp_entity_id`;
- `InResponseTo` = `request_id` (IdP-initiated disabled); `SubjectConfirmation` bearer with `Recipient` = ACS and
  `NotOnOrAfter` in the future; `Conditions` time window and `AudienceRestriction` = `sp_entity_id`; skew ≤ 120 s.

**After samael (verifier):** `Status` Success; the returned assertion's ID is the verified reference's; an
`AuthnStatement` is present; NameID non-empty.

**identity-login:** the state was valid and consumed; the path's connection is the state's; the connection is enabled
and allowed; the assertion ID is not in the replay cache (then stored until `NotOnOrAfter`); `SessionNotOnOrAfter`, when
present, caps the Sesame session.

## 6. Who the user is

- **Identity**: (`connection_id`, NameID, NameID format) in `saml_identities`, linked to a Sesame user. A persistent
  NameID is preferred; an email-format NameID is accepted (Google Workspace, Entra's default UPN).
- **First sign-in**: the email (the connection's `email_attribute`, else the NameID when it is an email) must be in one
  of the organisation's **verified** domains (ADR-007). Then:
  - an existing user of the tenant with that email, who is a member of the organisation, is linked;
  - otherwise, with just-in-time provisioning on, a user is created in the tenant and added to the organisation with
    the connection's default role (never an admin role);
  - otherwise refused (`not_provisioned`).
  Never linked through an unverified domain, never across tenants, never to a user who is not in the organisation.
- **Later sign-ins** use the link only; a changed email at the IdP updates nothing silently (audited).
- **Tokens** come from `issue_login_response` like every login: tenant and organisation from the connection, roles from
  authz-core, `amr: ["saml"]`, `acr` per ADR-008, session lifetime capped by the IdP's.
- **MFA**: the IdP's authentication is accepted for the organisation (an organisation using SSO owns its MFA); Sesame
  step-up (ADR-008) still applies to Sesame-sensitive operations.

## 7. Connection lifecycle

```
none ──allow (tenant admin)──► allowed ──metadata──► configured ──test sign-in ok──► tested ──enable──► enabled
                                  ▲                      │                                               │
                                  └──────── disable (tenant admin) / delete (org admin) ◄────────────────┘
```

- **allowed**: the tenant permits the organisation to use SAML.
- **metadata** (XML or HTTPS URL): org-mgmt fetches a URL (HTTPS only, ≤ 1 MiB, no redirects to other hosts, 10 s), has
  the verifier parse it, and stores entity ID, SSO URL (HTTP-Redirect), signing certificates (all of them, for rollover)
  and the raw XML. The SP metadata and the values the IdP admin needs (entity ID, ACS URL, NameID, attributes) are shown
  back (section 11).
- **test sign-in**: an org admin signs in through the connection with `test=true`; it proves the IdP configuration and
  the admin's own identity link, issues no tokens for the RP, and moves the connection to tested.
- **enabled**: members sign in with it. Hosted discovery offers it for the organisation's verified domains.
- **Refresh**: a connection with a metadata URL is re-fetched daily; new certificates are added, removed ones dropped
  after a 7-day overlap; a change of entity ID or SSO host is not applied automatically (audited, admin confirms).
  Certificates expiring within 30 days raise an alert to the organisation's admins.
- **Disable/delete**: immediately refuses new sign-ins; existing Sesame sessions continue to their normal expiry unless
  the admin also revokes them.

## 8. Data

```sql
ALTER TABLE sesame_idam.saml_connections
    ADD COLUMN tenant_id              VARCHAR(64),          -- RLS; backfilled from the organisation, then NOT NULL
    ADD COLUMN status                 VARCHAR(16) NOT NULL DEFAULT 'allowed',
    ADD COLUMN idp_entity_id          TEXT,
    ADD COLUMN idp_sso_url            TEXT,
    ADD COLUMN idp_signing_certs      JSONB NOT NULL DEFAULT '[]',   -- [{der_b64, not_after, added_at}]
    ADD COLUMN metadata_xml           TEXT,
    ADD COLUMN metadata_refreshed_at  TIMESTAMPTZ,
    ADD COLUMN email_attribute        TEXT,
    ADD COLUMN jit_provisioning       BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN default_role           VARCHAR(64),
    ADD COLUMN tested_at              TIMESTAMPTZ;
-- one connection per organisation; status IN ('allowed','configured','tested','enabled','disabled')

CREATE TABLE sesame_idam.saml_identities (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id      VARCHAR(64) NOT NULL,
    connection_id  UUID NOT NULL REFERENCES sesame_idam.saml_connections(id) ON DELETE CASCADE,
    name_id        TEXT NOT NULL,
    name_id_format TEXT NOT NULL,
    user_id        UUID NOT NULL REFERENCES sesame_idam.users(id) ON DELETE CASCADE,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_login_at  TIMESTAMPTZ,
    UNIQUE (connection_id, name_id)
);
```

Both tables get tenant RLS (ADR-005) and are written through Lifeguard models (`lifeguard-migrate` emits the migrations;
the hand-written parts are the RLS policies and the backfill). Redis keys (identity-login): `saml_state:<relay>`
(10 min), `saml_code:<code>` (60 s), `saml_assertion:<connection>:<assertion id>` (until `NotOnOrAfter`).

## 9. Testing

- **Verifier unit and corpus tests** (run in the verifier's image, CI-gating):
  - positive: real-shaped responses from Entra ID, Okta and Google Workspace test tenants (re-signed with test keys),
    Response-signed, Assertion-signed and both, RSA and ECDSA;
  - negative, one per rule in section 5: unsigned, wrong key, certificate only in `KeyInfo`, SHA-1, XSW1-8 variants,
    a second Assertion, comment in NameID, DOCTYPE/external entity/entity expansion, processing instruction, CDATA,
    `EncryptedAssertion`, wrong issuer, audience, recipient, destination, `InResponseTo`, expired, not yet valid, no
    `AuthnStatement`, non-Success status, oversize, bad base64;
  - parser differential: documents libxml2 and quick-xml might read differently (namespace redeclaration, duplicate
    attributes, entity references) must be refused or read identically.
- **identity-login**: state and code handling, replay, linking and provisioning rules (section 6), against a mock
  verifier; then end to end with the verifier and the local IdP of `pact-mock-server` (Tilt).
- **Contract**: the pending SAML contract tests become gating; sesame-idam-client's SAML calls move from pre-contract to
  contract.
- **Live acceptance** (pp): one test organisation per IdP; sign in to a relying party through each; disable, refresh,
  certificate rollover and replay exercised.
- **Upgrades**: every samael, libxmlsec1 or libxml2 bump runs the whole corpus (ADR-014 §4).
- **Fuzzing**: a fuzz target feeds mutated corpus responses to the verifier and fails on any accept the rules forbid.

## 10. Deployment

- **saml-verifier**: Deployment (2 replicas once production), ClusterIP Service, no Ingress; NetworkPolicy ingress only
  from identity-login-service and org-mgmt, egress none (not even DNS); non-root, read-only root file system, no service
  account token, CPU and memory limits, request body limit at the HTTP layer. Image built from its Dockerfile in the
  Octopilot pipeline (docker artifact).
- **identity-login-service**: public routes for section 4 on the deployment's public host; its SP key mounted from the
  secret backend (as the signing keyset); Redis as today.
- **org-mgmt**: egress to HTTPS metadata URLs (allow-listed per deployment where the platform supports it).
- **Configuration**: `SESAME_SAML_VERIFIER_URL`, `SESAME_SAML_SP_KEY_FILE`, `SESAME_SAML_MAX_CLOCK_SKEW_SECS` (default
  120), `SESAME_SAML_ENABLED` (off by default); public URLs and the claim namespace as for every login
  (DESIGN-octopilot-build-and-release §3.7).

## 11. IdP set-up (what the organisation's admin enters)

Sesame shows these per connection, with the SP metadata URL for IdPs that import it.

| | Entra ID (Enterprise application, SAML) | Okta (SAML 2.0 app) | Google Workspace (custom SAML app) |
| --- | --- | --- | --- |
| Identifier / Audience / Entity ID | SP entity ID | Audience URI (SP Entity ID) | Entity ID |
| Reply URL / SSO URL / ACS URL | ACS URL | Single sign-on URL (use for Recipient and Destination) | ACS URL |
| NameID | persistent or email | Persistent or EmailAddress | Primary email |
| Email attribute | `…/claims/emailaddress` (default) | add `email` → `user.email` | NameID, or add `email` |
| Signing | assertion (default) or both; SHA-256 | response and assertion; SHA-256 | assertion; response optional |
| Metadata for Sesame | App Federation Metadata URL | Metadata URL | download IdP metadata (XML) |

## 12. Order of work

1. **Verifier**: crate, Dockerfile, `/verify` and `/metadata/parse`, the corpus (section 9). Gate: corpus green.
2. **org-mgmt**: schema migration, organisation-scoped connection API (pending contract tests pass), metadata via the
   verifier, lifecycle (section 7), test sign-in.
3. **identity-login**: `saml_login` (signed AuthnRequest), ACS, `saml_callback`, SP metadata, discovery; replay; linking
   and provisioning; `amr`.
4. **Hosted UI**: "Sign in with SSO".
5. **Deployment** (pp): verifier and policies, SP key, configuration; live acceptance with the three IdPs.

## 13. Later

IdP-initiated login (unsolicited responses: its own replay and destination rules), single logout, encrypted assertions
(SP decryption key in the verifier would give it a secret: design first), signed metadata verification, SCIM
provisioning alongside SAML, more IdPs (they need only set-up rows in section 11 if they follow the profile).

## 14. xml-sec: audit (side project, not on the delivery path)

Reviewed 2026-10-10 at xml-sec 0.1.21 (`structured-world/xml-sec`). Decision (ADR-014 §6): libxmlsec1 is the verifier;
helping xml-sec to production readiness is a later side project, and adopting it would be a new ADR. The audit is kept
as the starting point for that work.

### 14.1 What it is

| | |
| --- | --- |
| Scope | XMLDSig sign/verify, XMLEnc, C14N 1.0/1.1/exclusive, X.509 path validation, key management, an XSLT 1.0 and XPath engine, a libxmlsec1-compatible CLI; about 133,000 lines of Rust |
| History | first commit 2026-03-15; 844 commits, 811 by one author; developed with coding agents (`AGENTS.md`); releases every few days |
| Licence | Apache-2.0 AND MIT (compatible with ours) |
| Rust | edition 2024, MSRV 1.92; crate-level `#![deny(unsafe_code)]`, vendored DOM/XPath crates built in no-unsafe variants |
| Crypto | RustCrypto (`rsa`, `p256`, `p384`, `p521`, `sha2`, …); optional aws-lc-rs FIPS provider |
| Stated status | "Early-stage pre-release. The API is unstable … should not yet be used in production." |
| Tests | interop and negative suites from libxmlsec1 ("donor") and the W3C Merlin vectors, golden C14N, fuzz targets (CI runs a 256-iteration smoke); `donor_negative_vectors`, `merlin_interop` and `donor_interop_suite` pass on 2026-10-10 |

### 14.2 Against what a SAML verifier needs

| Need | xml-sec | Assessment |
| --- | --- | --- |
| Enveloped-signature transform, exclusive C14N with `InclusiveNamespaces` | implemented | covers |
| RSA-SHA256/384/512 (PKCS#1 v1.5), ECDSA P-256/384 | implemented; XMLDSig `r‖s` ECDSA encoding by default | covers |
| Verify only with pinned keys, never `KeyInfo` | `RequireTrustedKey` is the default; `CallerTrustedSignatureKey` binds a key to one signature; embedded keys are candidates, not authority | covers, and by default |
| Consume exactly what was signed (signature wrapping) | `VerificationRequest.expected_targets` by node identity, `covers_element` proofs bound to the document generation; duplicate IDs never resolve | covers; stronger API than libxmlsec1's |
| SAML `ID` attributes | default ID indexing; explicit registrations as request context | covers (to confirm with our corpus) |
| No DTD, bounded parsing | DTDs off by default; byte, node, depth and parse-work ceilings checked before any DOM | covers |
| Parser differentials | optional differential mode parses with two backends (xmloxide, roxmltree) against the same limits | covers; unique among the options |
| Algorithm allow-list | typed, immutable policy | covers |
| SAML semantics (issuer, audience, conditions, subject confirmation) | out of scope (XMLDSig only) | ours, as with libxmlsec1 |
| Real IdP responses, SAML wrapping corpus | two SAML fixtures (ECDSA) | **gap**: our corpus fills it and goes upstream |
| Minimal build | the `xmldsig` feature also compiles DSA, DES, MD5, SHA-1, PKCS#12 code paths (policy-disabled) | **gap**: a SAML-verify feature profile, contributed |
| Independent review | none known; single author, agent-written | **gap**: our audit of the modules in scope |
| Continuous fuzzing | smoke only | **gap**: differential fuzzing against libxmlsec1 (section 9) |
| API stability | changes between minors (migration notes per release) | a gap for any production user |

Conclusion: the design is right for our problem; the evidence is not there yet. Worth investing in as a side project;
not something SAML delivery can wait for.

### 14.3 Risks

- **Unreviewed security code at high velocity**: the main reason it is not adopted now.
- **RustCrypto `rsa`**: its known timing weakness (RUSTSEC-2023-0071, Marvin) concerns private-key operations; a SAML SP
  verifies with public keys only. The SP's own signing (AuthnRequests) is not done with it (ring, ADR-014 §5).
- **Single maintainer**: any future adoption would need a fork we control.

### 14.4 If and when we take it up

A side project, separate from delivering SAML: contribute what a SAML verifier needs upstream (fixtures from the three
IdPs re-signed with test keys, the wrapping and parser-differential corpus, a minimal SAML-verify feature profile,
continuous differential fuzzing against libxmlsec1), audit the modules in scope (XML input and both parser backends,
ID resolution, XMLDSig parse, verify, URI dereferencing, the enveloped and exclusive-C14N transforms, C14N
serialisation, the caller-trusted key path, the policy compiler), and offer Sesame as a production user once upstream
is ready. Shadow, consensus or replacement in Sesame would each be a new ADR with its own evidence.
