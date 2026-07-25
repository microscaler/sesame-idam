# FINDING — `X-Tenant-ID` overrides the JWT in org-mgmt (2026-07-25)

> **Severity:** High. Cross-tenant read and write with a *valid* credential.
> **Found by:** asking how hauliage consumes sesame east-west vs how a SaaS
> tenant would consume it through the front door.

## What was found

`org-mgmt/impl/src/jwt_context.rs:22`:

```rust
pub fn tenant_from_request(req: &HandlerRequest) -> Option<String> {
    req.headers.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("x-tenant-id"))
        .map(|(_, v)| v.clone())
        .or_else(|| claims_from_request(req)?.get("tenant_id")...)
}
```

`or_else` is the whole finding. **The header is preferred; the verified JWT
claim is consulted only when the header is absent.** A caller holding a
legitimate token for tenant A sets `X-Tenant-ID: B` and org-mgmt acts on
tenant B.

This is the exact inversion of the rule. The signed claim is the one thing on
the request that cannot be forged; it is used as the fallback for the one
thing that can.

Affected — every authenticated org-mgmt operation:

| Controller | Effect across the boundary |
| --- | --- |
| `create_organization` | create an org inside another tenant |
| `invite_user_to_org` | **invite yourself into another tenant's org** |
| `fetch_users_in_org` | enumerate another tenant's users |
| `remove_user_from_org` | remove another tenant's users |
| `revoke_pending_invite` | revoke another tenant's invitations |

`invite_user_to_org` is the one to sit up about: it converts a read-across into
persistent membership.

## Why it survived review

The header is *correct* on identity-login-service — at login no token exists
yet, so the tenant must come from somewhere else. No other service has a
`jwt_context.rs`; org-mgmt copied a pre-authentication idiom into
post-authentication code, where the same line means something else entirely.

That is the ADR-011 boundary being decorative in the way the stock-take
predicted: the edge refuses to let a tenant name another tenant, and a service
one hop behind accepts it from a header.

## Not yet demonstrated

The hauliage BFF sets `X-Tenant-ID` from its own config, so a hauliage end user
cannot obviously reach this through the normal path. **Open question:** whether
BRRTRouter's proxy forwards an inbound `X-Tenant-ID` alongside the one the BFF
sets, producing two headers where `.find()` takes the first. Until that is
answered, treat exploitability through the BFF as unknown rather than absent.
Reachability from any pod in the cluster is not in doubt.

## Fix

After authentication, tenant comes from the token. Full stop. The header is
accepted only where no token exists (login, register), and where both are
present and disagree, **reject** — the pattern already implemented in
`identity-login-service/impl/src/services/tenant_admin.rs`, which was written
for exactly this and never applied here.

Tracked as **task 48**.
