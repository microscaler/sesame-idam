/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * OpenID Connect Discovery document (RFC 8414).
 * Returned by GET /.well-known/openid-configuration
 *
 * @example {
    "issuer": "https://identity.seasame-idam.microscaler.local",
    "authorization_endpoint": "https://identity.seasame-idam.microscaler.local/oauth/authorize",
    "token_endpoint": "https://identity.seasame-idam.microscaler.local/auth/token",
    "jwks_uri": "https://identity.seasame-idam.microscaler.local/.well-known/jwks.json",
    "userinfo_endpoint": "https://identity.seasame-idam.microscaler.local/oauth/userinfo",
    "scopes_supported": [
        "openid",
        "email",
        "profile",
        "phone"
    ],
    "response_types_supported": [
        "code",
        "id_token",
        "id_token token"
    ],
    "response_modes_supported": [
        "query",
        "fragment",
        "form_post"
    ],
    "grant_types_supported": [
        "authorization_code",
        "implicit",
        "refresh_token",
        "client_credentials",
        "urn:ietf:params:oauth:grant-type:token-exchange"
    ],
    "subject_types_supported": [
        "public",
        "pairwise"
    ],
    "id_token_signing_alg_values_supported": [
        "RS256"
    ],
    "code_challenge_methods_supported": [
        "S256"
    ]
}
 */
export type OpenIDConfiguration = {
    /**
     * Identity provider issuer identifier
     */
    issuer?: string;
    /**
     * OAuth2 authorization endpoint
     */
    authorization_endpoint?: string;
    /**
     * OAuth2 token endpoint
     */
    token_endpoint?: string;
    /**
     * JWKS endpoint for JWT verification
     */
    jwks_uri?: string;
    /**
     * OIDC userinfo endpoint
     */
    userinfo_endpoint?: string | null;
    /**
     * OAuth2 Dynamic Client Registration endpoint
     */
    registration_endpoint?: string | null;
    /**
     * Supported OAuth2 scopes
     */
    scopes_supported?: Array<string>;
    /**
     * Supported OAuth2 response types
     */
    response_types_supported?: Array<string>;
    /**
     * Supported OAuth2 response modes
     */
    response_modes_supported?: Array<string>;
    /**
     * Supported OAuth2 grant types
     */
    grant_types_supported?: Array<string>;
    /**
     * Supported subject types
     */
    subject_types_supported?: Array<string>;
    /**
     * Supported JWT signing algorithms (e.g. RS256)
     */
    id_token_signing_alg_values_supported?: Array<string>;
    /**
     * Supported userinfo signing algorithms
     */
    userinfo_signing_alg_values_supported?: Array<string>;
    /**
     * Supported userinfo encryption algorithms
     */
    userinfo_encryption_alg_values_supported?: Array<string>;
    /**
     * Supported userinfo encryption encodings
     */
    userinfo_encryption_enc_values_supported?: Array<string>;
    /**
     * Supported PKCE challenge methods
     */
    code_challenge_methods_supported?: Array<'S256' | 'plain'>;
};

