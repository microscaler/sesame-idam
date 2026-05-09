/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type TokenRequest = {
    /**
     * OAuth2 grant type
     */
    grant_type: 'refresh_token' | 'client_credentials' | 'urn:ietf:params:oauth:grant-type:token-exchange';
    /**
     * Refresh token (required for refresh_token grant)
     */
    refresh_token?: string;
    /**
     * Requested scopes (space-separated)
     */
    scope?: string;
    /**
     * Client ID (required for client_credentials and token exchange)
     */
    client_id?: string;
    /**
     * Client secret (required for client_credentials and token exchange)
     */
    client_secret?: string;
    /**
     * Subject token for token exchange (RFC 8693)
     */
    subject_token?: string;
    /**
     * Type of the subject token
     */
    subject_token_type?: 'urn:ietf:params:oauth:token-type:access_token' | 'urn:ietf:params:oauth:token-type:refresh_token';
    /**
     * Type of token being requested
     */
    requested_token_type?: 'urn:ietf:params:oauth:token-type:access_token' | 'urn:ietf:params:oauth:token-type:refresh_token';
};

