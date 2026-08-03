# Client ecosystem selection v1

Status: decided (2026-08-03)  
Provider profile: `1.0.x`  
Public API: `1.0.x`

## Mandated integration pattern

**BFF only.** End-user frontends must not call Sesame auth or tenant APIs
directly for credential exchange or refresh. The product backend (BFF or API
gateway it owns) is the Sesame relying party and tenant-consumer client.

Browser → product backend → Sesame (`id.` / `auth.` / `api.` public hosts).

No first-party browser SDK. No WASM auth client for SPAs.

## Selected for Supported investment

| Target | Tier | Package | Owner |
|---|---|---|---|
| Rust server client | **Supported** | [`sesame-idam-client`](https://github.com/microscaler/sesame-idam-client) (may-native today) | Sesame + Hauliage platform |
| Hauliage dogfood | **Supported** product path | Hauliage BFF/services using the Rust client + public contract | Hauliage |

Rust is the only language ecosystem with a Sesame-maintained client in this
selection. Multi-language SDKs (TypeScript/npm, Python, Java, Go, etc.) are
**deferred** until there is customer demand and a named maintainer.

## Deferred (not in Epic 16 active scope)

| Candidate | Prior idea | Disposition |
|---|---|---|
| Auth.js / Node BFF | Reference preset under `ui/reference-authjs` | Deferred; sample may remain but is not a release-blocking SDK |
| ASP.NET, Spring, Laravel, Rails, Go | Framework presets + API clients | Deferred |
| Python / Authlib | Helpers beyond Epic 15 proof | Deferred (Authlib remains Epic 15 contract proof only) |
| UniFFI / WASM / PyO3 bindings over Rust | Single core → many languages | Deferred; not justified while only Rust BFFs consume Sesame |
| Non-may async Rust client | Separate Tokio stack | Deferred; may client is the Supported path for microscaler |

## Package rule (when a future ecosystem is selected)

Any future package must remain thin over the ecosystem OIDC/JWT implementation
on the **server**, consume provider discovery, map only cryptographically
verified principals, support optional organization state, use the shared fixture
version, publish a supported-version matrix and provenance, and never add a
provider workaround. Provider incompatibilities are fixed in Sesame.

Frontends still use the BFF pattern; a future “JS client” would mean an Express
(or similar) server library, not a browser bundle.

## Dogfood expectation

Hauliage is the first real product on the portable OIDC/contract layer:

- existing email/password continues;
- add Google via Sesame social / upstream OIDC federation;
- Hauliage backend uses `sesame-idam-client` against public Sesame hosts and the
  Epic 15 contract (verified principal, tenant-consumer where applicable).
