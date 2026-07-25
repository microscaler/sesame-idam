# Note: the "artefact existed but did nothing" pattern

> **Status:** OBSERVATION (2026-07-25)
> **Prompted by:** "there is a lot of *the artefact existed but it did nothing*.
> This is due to the OpenAPI-first design… I still think OpenAPI-first is better
> than build-it-and-document-after."

The conclusion is right. The diagnosis is too narrow, and the narrow version
would lead to the wrong fix.

---

## 1. Tallying the actual findings

Nine instances of "it existed and did nothing" turned up in one session. Only
two are the OpenAPI-first failure mode.

| # | Finding | OpenAPI-first? |
| --- | --- | --- |
| 1 | `set_x(None)` emitted no SET clause; the old value survived | **No** — ORM type design |
| 2 | HTTPS redirect rule never fired (path specificity beat it) | **No** — Gateway API precedence |
| 3 | Rate limits guarded `/idam/auth/…` when the service serves `/idam/v1/auth/…` | Partly — wrong assumed path, not a spec problem |
| 4 | Unconfigured security scheme defaulted to the key `test123` | Partly — generator fallback policy |
| 5 | `ApiKeyHeader` declared in six specs, attached to no operation, used by no caller | **Yes** |
| 6 | `security: []` on all 11 authz-core operations | **Yes** |
| 7 | SMS/OAuth config behind a key no tenant could ever hold | **No** — nobody asked who holds it |
| 8 | NetworkPolicy in `k8s/`, which nothing reconciles | **No** — GitOps wiring |
| 9 | `git add -A` committed ten files as empty | **No** — tooling on shared storage |

Two squarely, two adjacent, five unrelated. Dropping OpenAPI-first would have
prevented **two** of these and left the other seven exactly as they were.

---

## 2. The actual common cause

Every one of the nine has the same shape:

> **A declaration and its enforcement live in different places, and nothing
> checks that they agree.**

- Spec says `security: BearerAuth`; enforcement lives in the router's provider
  registry. Nothing compares them. → 4, 5, 6
- HTTPRoute says "redirect"; enforcement lives in Gateway API's precedence
  rules. Nothing tests the rule actually wins. → 2
- `set_x(None)` declares an intention; emission decides whether a SET clause
  appears. `Option<T>` could not carry the distinction. → 1
- A NetworkPolicy file declares a restriction; Flux decides whether it exists.
  Nothing reconciled that directory. → 8
- A rate-limit policy declares a path; the service decides what path it serves.
  Nothing compared the two strings. → 3

This is not a spec-format problem. It is a **closed-loop** problem: intent is
written down in one system and enforced in another, and no third thing asserts
they match.

---

## 3. Why OpenAPI-first is still right — and the argument is stronger than "preference"

Code-first does not close the gap. It **hides** it.

If the spec is generated from the code, the two can never disagree — because
the spec has no independent content. It faithfully reports whatever the code
does, including the wrong thing. Under code-first:

- `authz-core` would still be unauthenticated, and the generated spec would
  cheerfully document eleven public endpoints as the intended design.
- There would be no `security: []` to grep for, because there would be no
  declaration to contradict reality. **The finding would not exist to be
  found.**

Findings 5 and 6 were discoverable *precisely because* an independent statement
of intent existed to compare against. Spec-first did not cause them; it is the
reason they were catchable at all.

What spec-first genuinely does is make declarations **cheap**, so there are
many of them, so unbacked ones accumulate faster. That is a real cost, and it
is an argument for verification, not for abandoning the method.

---

## 4. The fix: make declarations executable

Each declaration should have something that fails when reality diverges.

**Cheap, high value, mostly CI:**

1. **No `security: []` without a reviewed annotation.** A spec lint requiring
   `x-public-reason: "…"` on any operation declaring no auth. Would have caught
   finding 6 on the day it was written.
2. **Every declared scheme has a configured provider, or the service refuses to
   start.** The BRRTRouter uplift (task 34). Catches 4, and 5 becomes visible
   because an unused scheme is then an obvious startup warning.
3. **Generated conformance tests.** For every operation declaring a security
   scheme, a test asserting an unauthenticated request returns 401. Generated
   from the spec, so it cannot drift from it. Catches 5 and 6 permanently, and
   the whole class in future services.
4. **Path assertion for edge policy.** A test that every path named in a
   rate-limit or routing policy exists in the corresponding spec. Catches 3.
5. **A repo check that every manifest directory is reconciled by something.**
   Catches 8, which is otherwise invisible until an incident.

**Already fixed by construction:**

6. Finding 1 was fixed by making the type carry the distinction —
   `ActiveValue<T>` rather than `Option<T>`. That is the strongest form: the
   declaration and the enforcement became the same object, so they cannot
   disagree.

Item 6 is the pattern to prefer where it is available. Where it is not, item 3
— generating the test from the declaration — is the next best, because the
check is derived from the same source as the claim.

---

## 5. The heuristic worth keeping

Every one of the nine would have been caught by asking, at the time:

> **What would fail if this did nothing?**

If the answer is "nothing visible", the artefact is decorative until proven
otherwise, and the proof belongs in CI rather than in a reviewer's memory.

The variant that found two of them:

> **Who concretely does this, and what if the answer is nobody?**
