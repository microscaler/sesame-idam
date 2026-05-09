/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type SocialLoginResponse = {
    /**
     * JWT access token
     */
    access_token: string;
    /**
     * Always "Bearer"
     */
    token_type: SocialLoginResponse.token_type;
    /**
     * Access token lifetime in seconds
     */
    expires_in: number;
    /**
     * Refresh token
     */
    refresh_token: string;
    /**
     * UUID of the authenticated user
     */
    user_id: string;
    /**
     * User's email from the social provider
     */
    email?: string;
    /**
     * Whether the provider verified the email
     */
    email_verified?: boolean;
    /**
     * OAuth provider name (e.g., "github", "google")
     */
    social_provider: string;
    /**
     * User ID from the social provider
     */
    social_provider_user_id?: string;
};
export namespace SocialLoginResponse {
    /**
     * Always "Bearer"
     */
    export enum token_type {
        BEARER = 'Bearer',
    }
}

