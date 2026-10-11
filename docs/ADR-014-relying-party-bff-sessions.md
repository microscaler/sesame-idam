# ADR-014: Relying-party BFFs — hosted login, a confidential OIDC client, server-held sessions

> **Status:** PROPOSED (2026-10-11)
> **Builds on:** ADR-007 (domain verification), ADR-008 (assurance), ADR-010 (hosted auth surface, client SDK),
> ADR-013 (hosts and issuer), DESIGN-sesame-bff-and-consumer-migration.md
> **Scope:** how a relying party's browser application signs users in and keeps their session. Sesame's own consoles
> are covered by the Sesame BFF (task 54); server-to-server consumers by the API edge (task 50).

---

## 1. Context

ADR-010 settled where credentials are entered: on the hosted auth surface, never in a relying party's own form.
`POST /auth/session/code` exists so that a session can cross origins without a token ever appearing in a URL. What no
document settles is the other half: **what a relying party does with the session once it has it.**

Left open, integrations improvise, and the improvisation found in practice is the worst available: a relying party
posted credentials from its own form to its backend, received the access and refresh tokens, and handed them to its
app in a URL fragment, where the app kept them in `localStorage`. Every property ADR-010 was written to protect was
lost at once: credentials were collected outside the hosted surface (no passkeys, no MFA ceremony, no lockout
semantics), and a working session, refresh token included, travelled in a link that users copy to colleagues, that
lands in browser history and that every extension can read.

Two smaller gaps sit beside it:

- `@sesame/idam-client` redeems the authorization code without PKCE. The server already requires
  `code_challenge_method=S256` (`services/oidc_authorization.rs`), so this is a broken SDK path rather than a hole,
  but it is the path a single-page app would copy.
- The SDK keeps the session in `sessionStorage`, which any script on the page can read.

Sesame offers password, passkey, OTP, social (Google, GitHub) and SAML. If each method is integrated separately, each
needs its own session handling and each gets it slightly wrong. One integration has to serve all of them.

## 2. Decision

**A relying party's browser app never holds a Sesame token.** Its BFF is a confidential OIDC client that signs the
user in through the hosted auth surface, keeps the tokens server-side, and gives the browser an opaque, `HttpOnly`
session cookie. Every sign-in method ends in the same callback and the same session.

### 2.1 Every sign-in happens on the hosted auth surface

Password, passkey, email/phone OTP, magic link, social providers and SAML are all started and completed on
`auth.` (ADR-013) under the tenant's branding. The relying party shows a "Sign in" control that sends the browser to
its own BFF, nothing more. A relying party that renders a credential form or calls `/auth/login`, `/auth/register`
or `/auth/social/*` from its BFF on a user's behalf does not conform. (Those endpoints remain for the hosted surface
itself and for machine flows.)

SAML is chosen on the hosted surface by **home-realm discovery**: the user enters an email address; if its domain is
verified for an organisation with SAML enabled (ADR-007, org-mgmt `/sso/saml`), the surface sends the user to that
organisation's IdP (SP-initiated, HTTP-Redirect out, HTTP-POST back to the assertion consumer service) and provisions
the user just in time into that organisation. A verified domain with SAML enforced cannot fall back to password.

### 2.2 The BFF is a confidential client using authorization code with PKCE

```mermaid
sequenceDiagram
    autonumber
    participant U as Browser
    participant B as RP BFF (app.example.com)
    participant A as Sesame hosted surface (auth.)
    participant I as Sesame identity-login-service
    participant R as BFF session store

    U->>B: GET /auth/start?return_to=/dashboard
    B->>B: state, nonce, PKCE verifier (S256); keep in a short-lived pre-auth record
    B-->>U: 302 to auth./oauth/authorize?client_id&redirect_uri&state&nonce&code_challenge&method=S256&scope=openid profile email
    U->>A: sign in (password / passkey / OTP / Google / SAML via domain)
    A-->>U: 302 to app.example.com/auth/callback?code&state
    U->>B: GET /auth/callback?code&state
    B->>I: POST /oauth/token  code + code_verifier, client_secret_basic
    I-->>B: access, refresh, id_token
    B->>B: verify state; verify id_token (iss, aud, nonce, exp, signature via JWKS)
    B->>R: new session id (256-bit) -> tokens, subject, tenant, expiries
    B-->>U: 302 to return_to, Set-Cookie __Host-session=<id>; HttpOnly; Secure; SameSite=Lax; Path=/
```

- The PKCE verifier, `state` and `nonce` live only in the BFF's pre-auth record, keyed by `state`, expiring in
  minutes. The browser carries nothing but the code, which is single-use, bound to `redirect_uri` and useless
  without the verifier.
- Client authentication is `client_secret_basic` (supported today); `private_key_jwt` is the target (§6).
- `return_to` is a relative path validated against an allow-list; never an absolute URL (open redirect).
- `redirect_uri` is exact-match registered; one per BFF host.

### 2.3 The session is server-held; the cookie is opaque

- **Store:** the BFF's session store (Redis in practice) holds `{access, refresh, id_token, sub, tenant, org, created,
  last_seen, absolute_expiry}` under a random 256-bit id. Nothing derivable from the id.
- **Cookie:** `__Host-` prefixed (host-only, `Secure`, `Path=/`), `HttpOnly`, `SameSite=Lax`. The prefix forbids a
  `Domain` attribute, so a sibling host cannot read or set it. A relying party whose sign-in and app live on
  different hosts redirects to the app host's `/auth/start` instead of sharing a cookie across hosts.
- **Fixation:** a new id at every sign-in and privilege change (organisation switch, step-up); the old id is deleted.
- **Lifetime:** a sliding idle timeout refreshed by activity, and an absolute maximum after which the user signs in
  again (ADR-008 assurance may shorten it). Expiry deletes the record and revokes the refresh family.

### 2.4 The browser talks only to its BFF

The app calls same-origin `/api/...` on its BFF host. The BFF resolves the cookie, refreshes the access token when it
is within 60 s of expiry (one refresh in flight per session; Sesame's rotation and reuse detection then hold), and
forwards to its backends with `Authorization: Bearer`. Backends keep verifying Sesame JWTs via JWKS; nothing about
them changes. No route on the BFF host bypasses the BFF to reach a backend.

### 2.5 CSRF

`SameSite=Lax` stops cross-site sub-requests from carrying the cookie. In addition, every non-safe method must carry
an `Origin` on the BFF's allow-list and a custom request header (for example `X-Requested-With: sesame-bff`) that a
cross-site form cannot set. `/auth/callback` is protected by `state`.

### 2.6 Logout

`POST /auth/logout` revokes the refresh family at Sesame, deletes the session record and expires the cookie, then
sends the browser to the hosted surface's end-session endpoint so that the IdP session ends too (otherwise the next
sign-in is silent). "Sign out everywhere" revokes every session of the subject at this relying party. Back-channel
logout from Sesame to the BFF is the follow-up that makes an admin's revocation immediate (§6).

### 2.7 Organisations and teams

Invitations (org-mgmt `org_invites`) send the invitee to the hosted surface's sign-up with the invitation bound; on
first sign-in the user joins the organisation. The BFF reads organisation and role from the ID/access token claims;
switching organisation is `POST /sessions/active-organization` at Sesame followed by a session refresh and a new
session id (§2.3).

### 2.8 Apps without a BFF

A pure single-page app talking to Sesame directly is supported but discouraged. It must use authorization code with
PKCE S256 (the SDK is fixed to do so), keep tokens in memory only, and accept that a page reload means a silent
re-authentication (`prompt=none`) at the hosted surface. Relying parties that can run a backend use this ADR.

## 3. Shape in code

`sesame-idam-client` (Rust, may-native, used by BRRTRouter BFFs) gains a `bff` module:

| Piece | Responsibility |
| --- | --- |
| `BffConfig` | client id, secret reference, redirect URI, issuer, cookie name, idle/absolute timeouts, allowed origins and `return_to` prefixes |
| `SessionStore` trait, Redis implementation | pre-auth records (`state` → verifier, nonce, return_to) and sessions, with TTLs |
| `start_login` | builds the authorize redirect and its pre-auth record |
| `complete_login` | validates `state`, redeems the code with the verifier, verifies the ID token, creates the session, returns the `Set-Cookie` |
| `session_from_request` | cookie → session; touches `last_seen`; refreshes single-flight; yields the bearer token for downstream calls |
| `logout`, `logout_everywhere` | revocation, deletion, cookie expiry, end-session redirect |
| BRRTRouter middleware | CSRF checks (§2.5) and session resolution in front of every BFF route |

The existing password, registration and social helpers stay for Sesame's own hosted surface and are marked not for
relying-party BFFs. `@sesame/idam-client` gets PKCE S256 and in-memory storage (§2.8).

## 4. What exists and what this adds

| Piece | Today | This ADR adds |
| --- | --- | --- |
| `/oauth/authorize`, `/oauth/authorize/complete`, `/oauth/token` with S256 PKCE required, `nonce`, ID token | built (Epic 12) | relying parties actually use it |
| Confidential clients (`client_secret_basic` / `_post`), client secrets API | built (Epic 11) | relying-party BFFs register as confidential |
| Refresh rotation, reuse detection, family revocation | built | the BFF is the only refresher |
| `/auth/session/code` cross-origin handoff | built | unchanged; relying parties no longer need a handoff of their own |
| Hosted surface: password, OTP | built (ADR-010) | social and SAML entry points on it |
| Social login (`/auth/social/*`) | built, called by clients | called by the hosted surface only |
| SAML (org-mgmt `/sso/saml`, saml-verifier, domain claims) | built | home-realm discovery on the hosted surface; enforced SAML per verified domain |
| End-session endpoint | missing | RP-initiated logout ends the IdP session |
| `sesame-idam-client` `bff` module | missing | §3 |
| SDK PKCE, in-memory storage | missing | §2.8 |

## 5. Conformance

A relying party integration conforms when all of these hold, and its release tests assert them:

1. No URL, response body or response header outside `/oauth/token` (BFF to Sesame) carries an access, refresh or ID
   token.
2. The browser application contains no code that reads, stores or sends a Sesame token.
3. The session cookie is `__Host-` prefixed, `HttpOnly`, `Secure`, `SameSite=Lax`.
4. The authorize request carries `state`, `nonce` and an S256 `code_challenge`; the code exchange authenticates the
   client.
5. Copying any application URL into a different browser lands on sign-in.
6. A cross-site POST is rejected.
7. After logout, the old cookie and the old refresh token are both rejected.

## 6. Open questions

- `private_key_jwt` client authentication (keys in the relying party's KMS) instead of a shared secret.
- DPoP or mTLS-bound tokens between BFF and backends, so a token lifted from a BFF log is useless elsewhere.
- Back-channel logout to BFFs, so revocation in Sesame ends the relying party session within seconds.
- Whether relying parties may share a hosted-surface IdP session across products of one tenant (single sign-on
  between a tenant's apps) or must always prompt.
