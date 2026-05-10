/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type TokenRequest = {
    /**
     * OAuth2 grant type
     */
    grant_type: TokenRequest.grant_type;
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
    subject_token_type?: TokenRequest.subject_token_type;
    /**
     * Type of token being requested
     */
    requested_token_type?: TokenRequest.requested_token_type;
};
export namespace TokenRequest {
    /**
     * OAuth2 grant type
     */
    export enum grant_type {
        REFRESH_TOKEN = 'refresh_token',
        CLIENT_CREDENTIALS = 'client_credentials',
        URN_IETF_PARAMS_OAUTH_GRANT_TYPE_TOKEN_EXCHANGE = 'urn:ietf:params:oauth:grant-type:token-exchange',
    }
    /**
     * Type of the subject token
     */
    export enum subject_token_type {
        URN_IETF_PARAMS_OAUTH_TOKEN_TYPE_ACCESS_TOKEN = 'urn:ietf:params:oauth:token-type:access_token',
        URN_IETF_PARAMS_OAUTH_TOKEN_TYPE_REFRESH_TOKEN = 'urn:ietf:params:oauth:token-type:refresh_token',
    }
    /**
     * Type of token being requested
     */
    export enum requested_token_type {
        URN_IETF_PARAMS_OAUTH_TOKEN_TYPE_ACCESS_TOKEN = 'urn:ietf:params:oauth:token-type:access_token',
        URN_IETF_PARAMS_OAUTH_TOKEN_TYPE_REFRESH_TOKEN = 'urn:ietf:params:oauth:token-type:refresh_token',
    }
}

