# DESIGN: account-sharing controls for per-seat plans

Status: PROPOSED (October 2026) — for Charles to decide; nothing here is built yet.
Related: [DESIGN-mfa-passkeys.md](DESIGN-mfa-passkeys.md), `sesame_common::refresh_session` (one Redis record per
session, `rfam:{family_id}`, 26 Oct 2026), PriceWhisperer plans (Retail Trader $29, Professional $99, Desk $149 per
seat).

## 1. The honest part: second factors do not stop sharing

Two-step sign-in and passkeys stop *strangers* signing in with a stolen password. They do very little against a
subscriber who *wants* to share:

| Method | How a willing subscriber shares it |
|---|---|
| Password | tells a colleague |
| TOTP | scans the same QR code on two phones (or shares the base32 key; most apps also export) |
| Recovery codes | ten single-use passes; photocopiable |
| Synced passkey | Apple "Shared Passwords and Passkeys" groups (iOS 17+), Google Password Manager family sharing, 1Password/Bitwarden shared vaults: a passkey shared this way works on every member's device |
| Device-bound passkey (security key) | hand the key over between sessions |
| Magic link / emailed code | forward the email, or share the mailbox |

Second factors raise the *friction* of sharing (the phone has to be in the right hand at sign-in) and make casual
password sharing less convenient, which has some effect. They are not an enforcement mechanism, and selling them as
one would mislead Charles. What limits sharing is limiting **concurrent use**, which a per-seat plan can define
precisely: one seat = one person at a time.

## 2. What actually works

In order of value for cost:

1. **Concurrent session limit per seat** (per plan), with "sign out other devices" — the single most effective
   control; the Netflix/Spotify pattern users already understand.
2. **New-device notification** — "a new sign-in to your account from Chrome on Windows near Sofia". Makes sharing
   visible to the payer and catches account takeover too.
3. **Concurrent usage limit in the product** — the 9-concurrent-chart limit for Retail Trader: caps what one seat can
   consume however many people use it, and needs no identity work.
4. **Anomalous concurrent geography** — two sessions refreshing from places too far apart to be one person
   ("impossible travel"). A signal for review or step-up, never an automatic ban (VPNs and mobile carriers produce
   false positives).

### 2.1 Session limits

**What exists.** Every sign-in creates one refresh session, `rfam:{family_id}`, holding `sub`, `client_id`,
`tenant_id`, its absolute expiry and an idle TTL (8 h). Rotation is two Redis round trips. There is **no per-user
index**: counting a user's live sessions today would mean scanning every `rfam:*` key, which is not acceptable.

**Proposal.** A per-user set maintained at sign-in only, never on the rotation path:

* `usess:{tenant}:{sub}` — a sorted set: member `family_id`, score = absolute expiry. Added in the **same pipeline**
  as `refresh_session::create` (one extra command, `ZADD`, plus `EXPIRE` to the latest absolute expiry). Rotation,
  reuse detection and the grace window are untouched; they never read or write it.
* Counting at sign-in: `ZREMRANGEBYSCORE` (drop sessions past their absolute expiry), `ZRANGE`, then `EXISTS rfam:{f}`
  for each member in one pipeline (sessions that went idle, were logged out or revoked are gone from `rfam` and are
  pruned here). One extra round trip per sign-in; nothing per request.
* `sdev:{family_id}` — the session's device label (browser and OS family from the User-Agent, coarse location from the
  edge, created-at), written with the session, same TTL. Shown to the user; never used for authorisation.
* The limit comes from the plan: a principal attribute `max_sessions` (authz-core `principal_attributes` already
  exists) that PriceWhisperer's billing sets when the plan changes, read by identity-login at sign-in (the effective
  authz call it already makes). No attribute: no limit (retail tenants not using this are unaffected).

**Suggested limits** (Charles to set): Retail Trader 2 (desktop + phone), Professional 3, Desk 2 per seat.

**Where it is enforced.** At sign-in, in the gate that already decides tokens vs a second step
(`services::mfa`): when the user is at the limit, the sign-in answers `202` with a `session_limit` step instead of
tokens. Nothing changes for already-signed-in sessions, and the refresh path stays two round trips.

**User-facing behaviour** (hosted sign-in):

> **You're signed in on 2 devices — the most your Retail Trader plan allows.**
> Chrome on Windows · Sofia · active 5 minutes ago — *Sign out*
> Safari on iPhone · Sofia · active yesterday — *Sign out*
> [Sign out all other devices and continue] · [Cancel]

Choosing one revokes that session (`refresh_session::revoke`: its tokens stop refreshing within one access-token
lifetime, 5 minutes) and continues the sign-in. The removed device's next refresh fails and its BFF sends the browser
to sign-in with "You were signed out because your account was used on another device." A Desk admin sees seats and
active devices per member on the team page and can sign a member out everywhere.

Auto-evicting the oldest session silently (no prompt) is simpler but lets two sharers ping-pong unnoticed; the prompt
makes the sharing visible to both, which is the point.

### 2.2 New-device notification

* A long-lived random device cookie on the Sesame auth host (`__Host-sesame_device`, HttpOnly, SameSite=Lax, one year);
  its SHA-256 stored per user (`udev:{tenant}:{sub}`, a set with a 1-year TTL, at most 20 members).
* A sign-in without a known device cookie emails the account holder: device label, coarse location, time, and a
  "This wasn't me — sign out that session and reset my password" link (a single-use token that revokes the session).
* Cheap (one set lookup at sign-in), useful against takeover as well as sharing, and nothing for the honest user to do.

### 2.3 Concurrent chart limit (PriceWhisperer, not Sesame)

The 9-chart limit for Retail Trader is a product-usage limit and belongs in PriceWhisperer's market-data streaming
service, keyed by the token's `sub`:

* each open chart subscription holds a Redis key `charts:{sub}:{subscription}` with a 60 s TTL refreshed by the
  stream's heartbeat; opening one counts the user's live keys and refuses the 10th;
* the app shows: "You have 9 charts open (on 2 devices). Close one to open another." with a list of where they are open;
* per plan: Retail 9, Professional and Desk higher or unlimited (a plan attribute, as for `max_sessions`).

It caps what a shared Retail account is worth (one trader's screen) regardless of how many people log in, which makes
it the cheapest effective control and independent of everything above.

### 2.4 Anomalous concurrent geography

* Location: the GCP external HTTPS load balancer can add `X-Client-Geo-Location` (`{client_region},{client_city}`,
  `{client_city_lat_long}`) as a custom request header (gcp-infrastructure change; no GeoIP database or licence).
  The edge strips any client-supplied copy.
* At refresh, identity-session would compare the refreshing session's location with the user's other live sessions'
  last locations (`sdev:*`). That adds a read to the refresh path, which was just made two round trips; so instead
  compute it **at sign-in and on the new-device path only** (cheap) and, for live detection, in a background job
  sampling `sdev` updates. Recommended: start with sign-in-time checks only.
* Response: notify the account holder and flag the account for review; optionally require a fresh second-factor
  sign-in (ADR-008 step-up) on the next sensitive action. Never auto-ban.

## 3. Effort

| Piece | Where | Effort |
|---|---|---|
| `usess` index + `sdev` labels in `refresh_session::create` (one pipeline), live count at sign-in | sesame-common, identity-login | 2 days |
| Session-limit step in the gate + hosted UI (list, sign out, continue) | identity-login, frontend/auth | 3 days |
| `max_sessions` principal attribute + PriceWhisperer billing sets it on plan change | authz-core, PriceWhisperer billing | 2 days |
| Signed-out-elsewhere message in the trader BFF/app | PriceWhisperer | 1 day |
| New-device cookie + notification + "this wasn't me" revoke link | identity-login, frontend/auth | 2–3 days |
| Chart limit | PriceWhisperer market-data stream + trader UI | 2–3 days |
| LB geo header + sign-in-time impossible-travel check | gcp-infrastructure, identity-login | 2 days |
| Desk admin view of members' devices | org-mgmt, PriceWhisperer team page | 2–3 days |

Session limits + new-device emails + the chart limit: about two weeks, and they cover most of the value.

## 4. Decisions for Charles

1. Session limits per plan: 2 / 3 / 2-per-seat as suggested, or other numbers?
2. At the limit: **prompt** (recommended: visible, user chooses) or silently sign out the oldest?
3. Is the 9-chart Retail limit wanted as specified, and what are Professional's and Desk's?
4. New-device emails for everyone (recommended), or opt-in?
5. Geography: sign-in-time checks with notification only (recommended), or also step-up/flagging?
6. Should Desk admins be able to see and end members' sessions (privacy vs control)?
