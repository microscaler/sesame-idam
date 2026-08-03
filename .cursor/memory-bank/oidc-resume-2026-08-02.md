# OIDC / Epic resume — 2026-08-03

## Shipped

| Epic | Note |
|---|---|
| 14 | Conformance gate + product-neutral fixtures on `main` |
| 15 | `05bbb14` portable consumer contract; client `bb31009` |

## Epic 16 decision (locked)

- **BFF only** — frontends never do Sesame auth exchange/refresh
- **Rust only** Supported client: `sesame-idam-client`
- **Defer** Auth.js/npm, Python, Java, WASM/UniFFI bindings, other languages
- **Dogfood:** Hauliage — email/password + Google via Sesame social; public contract
- Active stories: 16.1 (done in selection doc), 16.2, 16.12, 16.11 (Rust-scoped)
- Selection: `docs/standards-first-oidc/client-ecosystem-selection-v1.md`
- INDEX / Epic 16 README → In progress

## Next implementation (when asked)

1. Hauliage: wire Google IdP through Sesame; align BFF to public hosts + verified principal
2. Client: public single-base / contract alignment as dogfood requires
3. CI: Rust client fixture/contract matrix (not cross-language)
