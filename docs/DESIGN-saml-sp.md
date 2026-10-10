# DESIGN: SAML service provider

Status: design (October 2026). Owner: Charles Sibbald.

Sesame signs users in through their organisation's SAML identity provider. First IdPs: **Microsoft Entra ID, Okta,
Google Workspace**; first flow: **SP-initiated** (the user starts at a Sesame relying party). IdP-initiated login,
single logout and encrypted assertions come later (section 9).

SAML is an upstream login method, like Google or Microsoft social login: the IdP vouches for a user, Sesame decides who
that user is in which tenant and organisation, and Sesame alone issues tokens. A relying party never sees a SAML message.

## 1. What exists

- `saml_connections` (org-mgmt): `org_id`, `issuer`, `metadata_url`, `sso_url`, `signing_cert`.
- org-mgmt operations for allow / enable / disable / link / metadata / delete: **stubs** (they audit and echo); their
  paths are not organisation-scoped (`/sso/saml/...`).
- `sesame-idam-client` already calls `GET /auth/saml/login` and `POST /auth/saml/callback`; the identity-login OpenAPI
  does not publish them (the client marks them pre-contract).
- Pending contract tests (`pact-mock-server/tests/saml_track_a_contract.rs`) describe the target paths; they are
  ignored until this lands and then gate it.
- Nothing parses or verifies SAML.

## 2. Flow (SP-initiated)

```
RP / hosted auth UI            Sesame identity-login                     Organisation IdP
  GET /auth/saml/login?org_id&redirect_uri&client_id
                       ──►  connection enabled? redirect_uri registered for client_id?
                            AuthnRequest(ID) signed (HTTP-Redirect binding), state in Redis (5 min, one use)
  ◄── 302 IdP SSO URL ?SAMLRequest&RelayState&SigAlg&Signature ─────────────────────►  user authenticates
                            POST /auth/saml/acs/{connection_id}  SAMLResponse, RelayState  ◄──
                            verify (section 4), resolve user (section 5)
  ◄── 302 redirect_uri?code=<saml_access_code>   (one use, 60 s, bound to client_id + redirect_uri)
  POST /auth/saml/callback {saml_access_code, client_id[, client_secret]}  (RP back end)
  ◄── TokenResponse (access, refresh, id token as for any login)
```

The hosted authorize flow uses the same machinery: "Sign in with SSO" on the login page takes an email, finds the
organisation whose **verified** domain (ADR-007) it is, and starts the login above with the pending authorize
transaction as its `redirect_uri`, so the RP receives an ordinary OIDC authorization code.

## 3. Endpoints

identity-login-service (public, `…/idam/v1`):

| Operation | Path | Notes |
| --- | --- | --- |
| `saml_login` | `GET /auth/saml/login` | `org_id`, `redirect_uri`, `client_id`; 302 to the IdP |
| `saml_acs` | `POST /auth/saml/acs/{connection_id}` | form post from the IdP (`SAMLResponse`, `RelayState`) |
| `saml_callback` | `POST /auth/saml/callback` | code exchange by the RP back end |
| `saml_sp_metadata` | `GET /auth/saml/sp/{connection_id}/metadata` | SP metadata for the IdP admin |

One SP entity per connection: entity ID `https://<public host>/idam/v1/auth/saml/sp/{connection_id}`, ACS as above.
Each organisation's IdP application is then independent, and the audience check names exactly one connection.

org-mgmt (organisation-scoped, `/organizations/{org_id}/sso/saml…`):

| Operation | Path | Who |
| --- | --- | --- |
| get / `delete_saml` | `GET`, `DELETE /organizations/{org_id}/sso/saml` | org admin |
| `allow_org_saml` / `disallow_org_saml` | `POST …/sso/saml/allow`, `…/disable` | tenant admin (entitlement) |
| `enable_saml` | `POST …/sso/saml/enable` | org admin, after a successful test login |
| `create_saml_link` | `POST …/sso/saml/link` | org admin: a one-time setup link for the org's IT admin |
| `set_saml_idp_metadata` | `PUT …/sso/saml/metadata` | org admin: IdP metadata XML or URL |

The stale unscoped `/sso/saml/*` paths go (the contract test forbids them).

## 4. Verifying a response

Every rule is mandatory; any failure rejects the login with an audit event and no detail to the browser.

- **Parsing.** `SAMLResponse` at most 256 KiB, strict base64. No DTD, no external entities, no XInclude (XXE).
- **Signature.** Required, on the Assertion or on a Response that contains exactly one Assertion. Verified only with the
  connection's pinned IdP certificates (from its metadata); a certificate in `KeyInfo` may select among them, never add
  one. Exactly one `Reference`, `URI="#<ID of the signed element>"`; transforms only enveloped-signature and exclusive
  C14N; algorithms RSA-SHA256/384/512 and ECDSA-SHA256/384 (SHA-1 rejected).
- **Signature wrapping.** The assertion consumed is the node the signature verified, located by the verified
  reference, never found again by searching the document; a second Assertion anywhere fails the response.
- **Message.** `Status` Success; `Issuer` = the connection's IdP entity ID; `Destination` (when present) = this ACS;
  `InResponseTo` = the pending request for this browser's `RelayState` (one use, consumed here; unsolicited responses
  are rejected until IdP-initiated login exists).
- **Assertion.** `SubjectConfirmation` method bearer with `Recipient` = this ACS, `NotOnOrAfter` in the future and the
  same `InResponseTo`; `Conditions` `NotBefore`/`NotOnOrAfter` with at most 120 s skew; `AudienceRestriction` contains
  the connection's SP entity ID; an `AuthnStatement`; `SessionNotOnOrAfter` caps the Sesame session.
- **Replay.** Assertion IDs are kept (Redis) until their `NotOnOrAfter`; a repeat is rejected.
- **NameID.** Compared and stored as received after XML normalisation; comments inside text nodes do not truncate it
  (the 2018 comment-injection class).

The verifier is one module with a corpus of real-shaped responses from the three IdPs (test tenants, test keys) and a
negative corpus: every rule above, the XSW1-8 wrapping variants, comment injection, DTD/XXE, SHA-1, two assertions.

## 5. Who the user is

- A user is identified by (`connection_id`, NameID) in `saml_identities` (new, RLS by tenant), linked to a Sesame user.
- First sign-in: the email attribute (configurable per connection; defaults per IdP below) must be in one of the
  organisation's verified domains. It links to an existing member of the organisation with that email, or, if the
  connection allows just-in-time provisioning, creates the user with the connection's default role. Never linked on an
  unverified domain, never across tenants.
- The token is issued as for any login: tenant and organisation from the connection, roles from authz-core, `amr`
  `saml`, session lifetime capped by the IdP's.

Per-IdP defaults (each is configurable in the IdP; the verifier accepts any of them, section 4):

| IdP | Signs by default | NameID by default | Email |
| --- | --- | --- | --- |
| Entra ID | Assertion (Response optional) | user principal name | `http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress` |
| Okta | Response and Assertion (configurable) | set per application | an `email` attribute statement |
| Google Workspace | Assertion (Response optional) | primary email | NameID, or an `email` attribute |

## 6. Data

`saml_connections` gains: `tenant_id` (RLS), `status` (`allowed`, `configured`, `enabled`, `disabled`),
`idp_entity_id`, `signing_certs` (several, for IdP key rollover), `metadata_xml`, `metadata_refreshed_at`,
`email_attribute`, `jit_provisioning`, `default_role`; one connection per organisation. New: `saml_identities`. The SP's
own key and certificate (AuthnRequest signing, SP metadata) is per deployment, from the secret backend (as ADR-006).

## 7. Library

Signature verification is the part that must not be written from scratch. Options (October 2026):

- **samael** with its `xmlsec` feature: SP-initiated SSO, metadata, verification through libxmlsec1 (the reference
  XML-DSig implementation behind most SAML stacks). Costs: C libraries (libxmlsec1, libxml2, libxslt) in the
  identity-login runtime image, so a run image beyond the default; the Rust binding is a modified copy maintained in
  samael (0.0.19, May 2025).
- **xml-sec** (pure Rust, exclusive C14N, RSA/ECDSA): no C dependencies, but it states it is not yet for production.
- Writing our own: rejected.

Recommendation: samael + libxmlsec1 for the signature, wrapped by our own verifier (section 4 owns every check that is
not the cryptographic signature), with the run image built and pinned like the builder. Revisit xml-sec when it is
declared production-ready. Decision recorded as an ADR before implementation.

## 8. Order

1. ADR (library, run image). Schema migration; org-mgmt connection API real and organisation-scoped (the pending
   contract tests pass); metadata parsing; SP metadata.
2. `saml_login`, ACS, `saml_callback`; the verifier with both corpora; identity linking; hosted "Sign in with SSO".
3. Live acceptance against Entra ID, Okta and Google Workspace test tenants, end to end through a relying party.

## 9. Later

IdP-initiated login (unsolicited responses, with its own replay and destination rules), single logout, encrypted
assertions, scheduled metadata refresh and certificate-rollover alerts, SCIM provisioning alongside SAML.
