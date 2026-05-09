/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type SocialCallbackRequest = {
    /**
     * Authorization code from the OAuth provider redirect
     */
    code: string;
    /**
     * CSRF state parameter (must match the one from login)
     */
    state: string;
    /**
     * Redirect URI used in the initial login request
     */
    redirect_uri?: string;
};

