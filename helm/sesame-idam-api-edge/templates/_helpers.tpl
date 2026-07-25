{{/*
These mirror helm/sesame-idam-frontend/templates/_helpers.tpl.

They are copied rather than shared because Helm has no way for two application
charts to import each other's templates — that needs a `type: library` chart and
a dependency, and a local `file://` dependency inside a Flux GitRepository source
is one more moving part on the path that has to actually reconcile. If a THIRD
chart ever needs these, that is the point to promote them to a library chart
rather than copy again. Until then: a change to the redirect or parentRefs lore
belongs in both files, and the comments say why in both.

`sesame.edge.filterList` below has no counterpart in the frontend chart — it is
the task 57 header surgery, and it emits list ITEMS rather than a whole
`filters:` block so a caller can prepend a URLRewrite (the issuer host does).
*/}}

{{/*
Gate A4 — the plaintext redirect rule.

Two things about Gateway API matching drive the shape of this, and both are easy
to get wrong:

  1. Rules are NOT tried in written order. Within a route, precedence goes to the
     most specific match — longest path prefix FIRST, and only then the number of
     header matches. A redirect rule matching `/` therefore LOSES to a serve rule
     matching `/idam/v1/auth/login`, and the redirect silently never fires. So
     the redirect must match the SAME paths as the rule it needs to pre-empt, and
     win the tie on the extra header match.
  2. Precedence is per-route. A redirect on one route does nothing for traffic
     that matched a more specific route, so EVERY route below carries its own
     copy — which is why this is a helper and not a rule written once.

Matched on X-Forwarded-Proto rather than the listener because haproxy terminates
TLS and forwards to Envoy on :80 — the listener reports "http" for a caller that
spoke perfectly good HTTPS, and redirecting on it would loop until the client
gave up.

No header filters here on purpose: the response is a 301 and no backend is
reached, so there is nothing to strip. The strip belongs on the serve rule.

Usage: {{- include "sesame.edge.httpsRedirectRule" (dict "ctx" $ "paths" $g.paths) | nindent 4 }}
*/}}
{{- define "sesame.edge.httpsRedirectRule" -}}
{{- $ctx := .ctx -}}
{{- if $ctx.Values.route.tls.redirectToHttps }}
- matches:
{{- range $p := .paths }}
    - path:
        type: PathPrefix
        value: {{ $p | quote }}
      headers:
        - name: X-Forwarded-Proto
          value: http
{{- end }}
  filters:
    - type: RequestRedirect
      requestRedirect:
        scheme: https
        # 301: permanent. Safe because the HTTPS listener is a fixture of the
        # Gateway, not something a rollout takes away.
        statusCode: 301
{{- end }}
{{- end -}}

{{/*
The Gateway parentRefs block.

A listener is pinned by `sectionName`, and a listener carries exactly ONE
hostname — so which listener a route attaches to is not cosmetic: a route whose
`hostnames` are not covered by the named listener's hostname attaches to nothing
and reports Accepted=False.

Each of `route.gateway.sectionNameHttp` / `sectionNameHttps` may be a single name
or a LIST of names. A list is the safe way to move zones: point at the old and
the new listener at once, cut the hostname over, then drop the old name — at no
point is the route detached.

Usage: {{- include "sesame.edge.parentRefs" $ | nindent 4 }}
*/}}
{{- define "sesame.edge.parentRefs" -}}
{{- $gw := .Values.route.gateway -}}
{{- $sections := list -}}
{{- range $v := (list $gw.sectionNameHttps $gw.sectionNameHttp) }}
{{- if kindIs "slice" $v }}{{- $sections = concat $sections $v }}
{{- else if $v }}{{- $sections = append $sections $v }}
{{- end }}
{{- end }}
{{- range $s := $sections }}
- group: gateway.networking.k8s.io
  kind: Gateway
  name: {{ $gw.name }}
  namespace: {{ $gw.namespace }}
  sectionName: {{ $s | quote }}
{{- end }}
{{- end -}}

{{/*
Task 57 + Gate A4 — the filters every serve rule on `api.` and `id.` carries.

Emitted as list ITEMS (no `filters:` key) so the issuer host can prepend a
URLRewrite to the same list.

WHY THE REQUEST STRIP (Cookie)
------------------------------
The API authenticates SOLELY from `Authorization`. A cookie on an API request is
therefore never load-bearing and always ambient — authority the browser attached
because of where the request was going, not because the caller proved anything.

ADR-013 abandoned the two-registrable-domain split, whose real argument was that
a browser CANNOT attach a console session cookie to an API request. Stripping at
the edge is strictly stronger than that split:

  - it does not depend on browser cookie scoping, `__Host-` prefixes, or any
    developer remembering not to set `Domain=`
  - it holds for NON-BROWSER callers too, which is most of this API's traffic and
    which the domain split never covered at all
  - it is TESTABLE. "A request carrying a session cookie and no bearer gets 401"
    is a conformance test rather than an argument

It also blunts the one thing ADR-013 made worse: on a single registrable domain,
a dangling subdomain (task 58) becomes a way to set cookies scoped to the parent.
Those cookies now reach nothing.

WHY THE RESPONSE STRIP (Set-Cookie)
-----------------------------------
Decided yes, and for `id.` it is not optional.

Nothing routed here sets a cookie today — there is no `Set-Cookie` anywhere in
microservices/idam. So this removes no behaviour. What it removes is a FUTURE:
a framework default, a session middleware added for a console feature, or a
library that "helpfully" pins a load-balancer affinity cookie. Any of those would
mint ambient authority on the API origin, which is precisely the thing the
request strip exists to stop, arriving from the other direction — and it would
arrive silently, because a cookie that works looks like nothing at all.

For `id.` it is a correctness requirement rather than hygiene: ADR-013 says the
issuer host carries "none, ever", because discovery and JWKS are fetched by every
relying party forever and must be cacheable by anything in the path. A response
carrying `Set-Cookie` is not safely shared-cacheable, and a cookie on the issuer
origin is ambient authority attached to a document whose whole point is that it
carries none.

The cost of being wrong in this direction is a loud, immediate failure (a login
that does not stick) rather than a silent, permanent one. That asymmetry is the
whole argument.

WHY X-Tenant-ID
---------------
See values.yaml `edge.stripTenantHeader`. Same principle, different header: the
verified claim is the thing that cannot be forged, so the forgeable twin must not
survive the boundary.

Usage:
  filters:
{{- include "sesame.edge.filterList" $ | nindent 8 }}
*/}}
{{- define "sesame.edge.filterList" -}}
{{- $e := .Values.edge -}}
{{- $tls := .Values.route.tls -}}
{{- $removeReq := list -}}
{{- if $e.stripCookie }}{{- $removeReq = append $removeReq "Cookie" }}{{- end }}
{{- if $e.stripTenantHeader }}{{- $removeReq = append $removeReq "X-Tenant-ID" }}{{- end }}
{{- if $removeReq }}
- type: RequestHeaderModifier
  requestHeaderModifier:
    remove:
{{- range $h := $removeReq }}
      - {{ $h }}
{{- end }}
{{- end }}
{{- if or $e.stripSetCookie $tls.hstsMaxAgeSeconds }}
- type: ResponseHeaderModifier
  responseHeaderModifier:
{{- if $e.stripSetCookie }}
    remove:
      - Set-Cookie
{{- end }}
{{- if $tls.hstsMaxAgeSeconds }}
    set:
      - name: Strict-Transport-Security
        value: "max-age={{ $tls.hstsMaxAgeSeconds }}{{ if $tls.hstsIncludeSubdomains }}; includeSubDomains{{ end }}"
{{- end }}
{{- end }}
{{- end -}}

{{/*
Gate A1 — a per-route local rate-limit policy.

Envoy Gateway attaches a BackendTrafficPolicy to a WHOLE HTTPRoute, not to an
individual rule, so "a different budget per path" means "a route per path". That
is the only reason the routes below are split the way they are.

Keyed on client IP, which only means anything because the Gateway trusts the LAN
proxy's X-Forwarded-For (ClientTrafficPolicy `trust-lan-proxy-xff` in
shared-gitops). Without that every caller shares haproxy's address and the first
flood locks out everyone.

`type: Local` is per Envoy pod. One data-plane replica today, so the numbers are
exact; scaling the proxy multiplies them, and the answer then is a global
(Redis-backed) limit, which changes `type` and nothing else here.

Usage: {{- include "sesame.edge.rateLimitPolicy" (dict "ctx" $ "name" $name "limit" $g.limit) }}
*/}}
{{- define "sesame.edge.rateLimitPolicy" -}}
{{- $ctx := .ctx -}}
apiVersion: gateway.envoyproxy.io/v1alpha1
kind: BackendTrafficPolicy
metadata:
  name: {{ .name }}
  namespace: {{ $ctx.Values.namespace }}
  labels:
    app: sesame-api-edge
    app.kubernetes.io/part-of: sesame-idam
    sesame.microscaler.io/gate: "A1"
spec:
  targetRefs:
    - group: gateway.networking.k8s.io
      kind: HTTPRoute
      name: {{ .name }}
  rateLimit:
    type: Local
    local:
      rules:
        - clientSelectors:
            - sourceCIDR:
                # Every client, counted individually — the selector exists to
                # make the bucket per-IP, not to pick out a subnet.
                value: 0.0.0.0/0
                type: Distinct
          limit:
            requests: {{ .limit }}
            unit: Minute
{{- end -}}
