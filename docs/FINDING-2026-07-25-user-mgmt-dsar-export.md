# FINDING — unauthenticated GDPR DSAR export (2026-07-25)

> **Severity:** High. PII disclosure. Not externally routed; reachable by any
> pod in the shared cluster.
> **Found by:** the task 45 spec lint, on its first run.

## What was found

Three `identity-user-mgmt-service` operations declared `security: []`:

| Path | Operation |
| --- | --- |
| `/admin/audit/users/{user_id}/events/compliance-export` | `export_user_audit_events` |
| `/admin/audit/events` | `get_user_audit_events` |
| `/admin/audit/users/{user_id}/events/count` | `get_user_event_count` |

The first is described in the spec as *"Export all audit events for a specific
user — required for GDPR Data Subject Access Requests."* It takes `user_id`
from the path and required no credential.

**That is the data a DSAR exists to release only to its subject, available to
anyone who could reach the service, for any user they can name.**

The `/admin/` prefix was doing the same job `/platform/` was doing in ADR-011:
implying an access-control decision that nothing enforced.

## Why it was missed

Same cause as the authz-core finding: `security: []` reads as a declaration
rather than an absence, and nothing compared it against intent. It is the
second instance of the pattern in
[NOTE-declaration-enforcement-gap.md](./NOTE-declaration-enforcement-gap.md),
found within minutes of building the check for the first.

## Fix

Now `BearerAuth`, chosen because it is the only provider this service
registers at startup — verified in the running pod's logs. Declaring an
unconfigured scheme would fall back to BRRTRouter's static `test123` key and
look fixed without being (task 34).

**Not yet live**: needs a rebuild and redeploy of identity-user-mgmt-service.

## Follow-up

Authentication is not authorization. Requiring a bearer stops anonymous
access, but any valid token is currently sufficient — a signed-in user of one
tenant could still request another user's export. These are `/admin/`
operations and need an operator-or-self authorization check in the controller,
which is task 41 territory.

Tracked as **task 47**.
