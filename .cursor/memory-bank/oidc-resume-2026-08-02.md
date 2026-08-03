# OIDC / Epic 14 — 2026-08-03

## Open-source fixture naming (product brands removed)

Product-specific symbols are gone from Sesame test/seed code:

| Old | New |
|-----|-----|
| `HAULIAGE_TENANT` | `FIXTURE_TENANT` = `"acme"` |
| `HAULIAGE_WEB_CLIENT` | `FIXTURE_WEB_CLIENT` = `"acme-web"` |
| `owner@hauliage.dev` | `owner@acme.example` |
| seed `*_hauliage_demo_*.sql` | `*_acme_demo_*.sql` |
| tenant `pricewhisperer` | `globex` |
| `SESAME_OAUTH__PRICEWHISPERER__*` | `SESAME_OAUTH__GLOBEX__*` |

Live/private lab overrides (keep Hauliage out of the OSS tree):

```bash
export SESAME_LIVE_TEST_TENANT=hauliage          # private only
export SESAME_LIVE_TEST_CLIENT_ID=hauliage-web
export SESAME_LIVE_TEST_REDIRECT=https://loadlinker.dev.microscaler.local/auth/callback
export SESAME_LIVE_TEST_EMAIL=owner@hauliage.dev
```

After seed rename, re-apply platform + demo seeds (or rely on env overrides against existing DB rows).

## Epic 14 status
See prior notes; conformance BDD green with `acme` fixture tenant strings.
