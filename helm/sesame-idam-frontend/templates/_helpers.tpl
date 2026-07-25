{{/*
Gate A4 — the plaintext redirect rule, defined once.

Two things about Gateway API matching drive the shape of this, and both are
easy to get wrong:

  1. Rules are NOT tried in written order. Within a route, precedence goes to
     the most specific match — longest path prefix first, and only then the
     number of header matches. A redirect rule matching `/` therefore LOSES to
     a serve rule matching `/idam/v1/auth/login`, and the redirect silently
     never fires. So the redirect must match the SAME paths as the rule it
     needs to pre-empt, and win the tie on the extra header match.
  2. Precedence is per-route. A redirect on the catch-all route does nothing
     for traffic that matched a more specific route, so every route that can
     match an auth path needs its own copy.

Matched on X-Forwarded-Proto rather than the listener because haproxy
terminates TLS and forwards to Envoy on :80 — the listener reports "http" for
a browser that spoke perfectly good HTTPS, and redirecting on it would loop
until the browser gave up.

Usage: {{- include "sesame.httpsRedirectRule" (dict "ctx" $ "paths" (list "/")) | nindent 4 }}
*/}}
{{- define "sesame.httpsRedirectRule" -}}
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
Gate A4 — the HSTS response header, defined once.

Emitted as a `filters:` block, so it is only valid on a rule that has no other
filters. Both call sites satisfy that.
*/}}
{{- define "sesame.hstsFilter" -}}
{{- if .Values.route.tls.hstsMaxAgeSeconds }}
filters:
  - type: ResponseHeaderModifier
    responseHeaderModifier:
      set:
        - name: Strict-Transport-Security
          value: "max-age={{ .Values.route.tls.hstsMaxAgeSeconds }}{{ if .Values.route.tls.hstsIncludeSubdomains }}; includeSubDomains{{ end }}"
{{- end }}
{{- end -}}
