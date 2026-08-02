import type { OAuthConfig, OAuthUserConfig } from "@auth/core/providers"

export interface SesameProfile {
  sub: string
  email?: string
  email_verified?: boolean
  name?: string
  preferred_username?: string
}

export interface SesameOptions extends OAuthUserConfig<SesameProfile> {
  issuer: string
  clientId: string
  clientSecret?: string
}

/**
 * Thin Auth.js preset. Auth.js owns discovery, PKCE/state/nonce generation,
 * code exchange, and ID-token validation; Sesame supplies only defaults and
 * profile mapping.
 */
export function Sesame(options: SesameOptions): OAuthConfig<SesameProfile> {
  return {
    id: "sesame",
    name: "Sesame",
    type: "oidc",
    issuer: options.issuer,
    clientId: options.clientId,
    clientSecret: options.clientSecret,
    checks: ["pkce", "state", "nonce"],
    authorization: {
      params: { scope: "openid profile email" },
    },
    client: {
      token_endpoint_auth_method: options.clientSecret
        ? "client_secret_basic"
        : "none",
    },
    profile(profile) {
      return {
        id: profile.sub,
        email: profile.email ?? null,
        name: profile.name ?? profile.preferred_username ?? profile.email ?? null,
        image: null,
      }
    },
    options,
  }
}
