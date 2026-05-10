/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * OpenID Connect Discovery document (RFC 8414).
 * Returned by GET /.well-known/openid-configuration
 *
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

