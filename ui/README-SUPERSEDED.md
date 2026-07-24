# superseded by ../frontend/

`ax-frontend`, `cx-frontend` and `brochure` here were bare Solid-Vite template
scaffolds. Per ADR-010 the Sesame UI lives in `../frontend/`:

| old              | new                  |
|------------------|----------------------|
| `ax-frontend`    | `frontend/platform`  |
| `cx-frontend`    | `frontend/tenant`    |
| `brochure`       | `frontend/brochure`  |
| —                | `frontend/auth` (hosted auth surface) |
| —                | `frontend/shared`, `frontend/client-sdk` |

Do not add work here. `images/` and `test/` are untouched.
