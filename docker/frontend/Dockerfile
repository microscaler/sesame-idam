# Static build for any Sesame frontend app (ADR-010).
#
# One Dockerfile, four images — the app is selected by APP build-arg:
#   docker build -f docker/frontend/Dockerfile --build-arg APP=auth .
#
# Build context is the REPO ROOT (the pnpm workspace lives in frontend/ and
# apps depend on frontend/shared + frontend/client-sdk via workspace links).
FROM node:22-alpine AS build
ARG APP
# VARIANT=debug (pp) ships source maps, so browser errors and profiles map back to the source; VARIANT=release (prod)
# ships none. skaffold.yaml builds both as <image>-debug and <image>-release.
ARG VARIANT=release
WORKDIR /build

RUN corepack enable && corepack prepare pnpm@9.12.0 --activate

# Manifests first for layer caching.
COPY frontend/package.json frontend/pnpm-workspace.yaml ./
COPY frontend/shared/package.json ./shared/
COPY frontend/client-sdk/package.json ./client-sdk/
COPY frontend/auth/package.json ./auth/
COPY frontend/platform/package.json ./platform/
COPY frontend/tenant/package.json ./tenant/
COPY frontend/brochure/package.json ./brochure/
RUN pnpm install --no-frozen-lockfile

COPY frontend/ ./
RUN if [ "$VARIANT" = debug ]; then pnpm --filter "@sesame/${APP}" build --sourcemap; else pnpm --filter "@sesame/${APP}" build; fi

# ── runtime: nginx serving the SPA ──────────────────────────────────────────
FROM nginx:1.27-alpine AS runtime
ARG APP
COPY --from=build /build/${APP}/dist /usr/share/nginx/html
COPY docker/frontend/nginx.conf /etc/nginx/conf.d/default.conf
COPY docker/frontend/idam-proxy.inc /etc/nginx/conf.d/idam-proxy.inc

# Non-root (chart runs with the platform's default context).
RUN touch /var/run/nginx.pid \
 && chown -R nginx:nginx /var/run/nginx.pid /var/cache/nginx /usr/share/nginx/html
USER nginx
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://127.0.0.1:8080/healthz || exit 1
