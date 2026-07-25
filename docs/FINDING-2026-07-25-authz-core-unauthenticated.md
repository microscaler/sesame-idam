# FINDING — authz-core is entirely unauthenticated (2026-07-25)

> **Severity:** High. Not externally exposed; reachable by **any pod in the
> shared cluster**, which hosts other products.
> **Found by:** Task 38 (ADR-012 Wave 0), asking whether `authz-core` checks
> its caller before honouring `x-tenant-id`.
> **Answer:** It does not check the caller at all.

---

## 1. What was found

All **11 operations** in `authz-core` declare `security: []` in the OpenAPI
document — which is not "inherit the default", it is the explicit OpenAPI
spelling of *no authentication required*. It overrides the document-level
`security` block.

| Operation | Effect if called by anyone |
| --- | --- |
| `principal_effective` | read any principal's roles, permissions and attributes, **in any tenant** |
| `list_audit_events` | read any tenant's audit log |
| `search_audit_events` | search any tenant's audit log |
| `get_audit_event` | read a specific audit event |
| `get_audit_stats` | audit volume metadata |
| `export_audit_events` | **bulk export** any tenant's audit trail |
| `check_export_status` | track that export |
| `list_retention_policies` | read retention configuration |
| `create_retention_policy` | **write** |
| `update_retention_policy` | **write** |
| `delete_retention_policy` | **write — remove audit retention** |

The tenant is taken from the request body (`req.data.tenant_id`) and used
directly, so there is no parameter binding the answer to any authenticated
identity. This is ADR-012 §2.3's prohibited pattern in its purest form.

## 2. Proof

A pod in an unrelated namespace (`loadlinker/analytics`) called
`authz-core.sesame-idam.svc.cluster.local:8080` with no credential:

```
wget --post-data='{"tenant_id":"hauliage","principal_id":"…"}' \
  http://authz-core.sesame-idam.svc.cluster.local:8080/idam/v1/authz/principals/effective
→ HTTP/1.1 400 Bad Request
```

**400, not 401.** The request was accepted for processing and rejected on
payload shape. Nothing asked who was calling.

## 3. Exposure

- **Not externally routable.** No HTTPRoute; the Service is `ClusterIP`, and
  the frontend nginx proxies only to `identity-login-service`.
- **Reachable cluster-wide.** Four NetworkPolicies exist, covering `redis` and
  `flux-system`. Nothing restricts `sesame-idam`. Any workload in any namespace
  can reach it — and this cluster hosts other products.

So the realistic attacker is a compromised or malicious workload elsewhere in
the cluster, or anything with `kubectl exec` into any pod. Not the internet.

## 4. Why it matters more than it looks

The audit endpoints are the part to sit up about. `export_audit_events` is bulk
exfiltration of the record of who did what, across tenants. `delete_retention_policy`
lets an unauthenticated caller change how long that record is kept — which is
anti-forensics: an attacker who reaches this service can arrange for the
evidence of everything else to be discarded.

This also undermines Gate C before it is built. C1 ships audit logs off-cluster
and C4 requires a compromise to be reconstructable; both assume the audit trail
is trustworthy. An unauthenticated write path to retention policy means it is
not.

And it makes ADR-011's boundary partly decorative: the edge carefully refuses
to let a tenant name another tenant, while a service one hop behind will answer
questions about any tenant for anyone who asks.

## 5. Fix

**Immediate (hours), in order of speed:**

1. **NetworkPolicy** restricting `authz-core` ingress to the pods that call it
   (`identity-login-service`, and whatever else appears in a real audit of
   callers). This is Gate B4, already on the tracker, and it removes "any pod
   in the cluster" without touching application code. Topology is not identity,
   but it collapses the practical blast radius today.
2. **Remove `security: []`** from all 11 operations so they inherit a real
   scheme. This is a spec change plus regeneration — cheap, and it is the
   actual fix rather than a mitigation.

**Correct (per ADR-012):**

3. `principal_effective` should take a **delegated user token** and derive the
   subject and tenant from it, rather than accepting `tenant_id` in the body at
   all — making §2.3 structural for this service instead of a rule to remember.
   This is ADR-012 §6's open question, now answered: yes.
4. The audit endpoints are **platform-operator** surface, not service-to-service.
   They belong behind per-operator identity (task 41), and read/export should be
   separable from retention-policy mutation.

**Do not** simply attach `ApiKeyHeader` — that scheme currently resolves to the
static `test123` fallback (task 34), so it would look fixed and not be.

## 6. Tests this must not pass again

1. Every operation refuses an unauthenticated request with **401**, not 400.
2. A caller authenticated for tenant A cannot obtain effective permissions for
   tenant B.
3. Retention-policy mutation requires operator identity, and is refused for a
   service credential.
4. No operation in any spec carries `security: []` without an explicit,
   reviewed comment saying why — the other 27 across five specs need the same
   audit (identity-login-service has 18, largely legitimate public auth
   endpoints, but they have never been reviewed as a set).
