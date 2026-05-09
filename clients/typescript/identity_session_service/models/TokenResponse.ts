/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type TokenResponse = {
    /**
     * JWT access token (valid for 15 minutes by default)
     */
    access_token: string;
    /**
     * Always "Bearer"
     */
    token_type: 'Bearer';
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

