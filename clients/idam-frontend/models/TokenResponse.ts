/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * @example {
    "access_token": "eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiIxMjMiLCJlbWFpbCI6ImFsaWNlQGV4cC5jb20iLCJvcmdfaWQiOiIxMTg5YzQ0NCJ9.sig",
    "token_type": "Bearer",
    "expires_in": 900,
    "refresh_token": "cmVmcmVzaC10b2tlbi1hbGljZS1zZXNzaW9u",
    "refresh_token_expires_in": 2592000,
    "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
    "email": "alice@example.com",
    "email_verified": true,
    "phone_verified": false,
    "mfa_required": false,
    "id_token": null,
    "scope": "openid profile email"
}
 */
export type TokenResponse = {
    /**
     * JWT access token (valid for 15 minutes by default)
     */
    access_token: string;
    /**
     * Always "Bearer"
     */
    token_type: TokenResponse.token_type;
    /**
     * Access token lifetime in seconds (default: 900)
     */
    expires_in: number;
    /**
     * Refresh token for obtaining new access tokens
     */
    refresh_token: string;
    /**
     * Refresh token lifetime in seconds (default: 2592000 = 30 days)
     */
    refresh_token_expires_in?: number;
    /**
     * UUID of the authenticated user
     */
    user_id: string;
    /**
     * User's email address
     */
    email?: string | null;
    /**
     * Whether the email has been verified
     */
    email_verified?: boolean;
    /**
     * Whether the phone has been verified
     */
    phone_verified?: boolean;
    /**
     * Whether MFA is required for this user
     */
    mfa_required?: boolean;
    /**
     * OpenID Connect ID token (present in OIDC flows)
     */
    id_token?: string | null;
    /**
     * Granted OAuth scopes
     */
    scope?: string | null;
};
export namespace TokenResponse {
    /**
     * Always "Bearer"
     */
    export enum token_type {
        BEARER = 'Bearer',
    }
}

