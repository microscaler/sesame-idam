/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Response containing OAuth provider redirect URL and CSRF state token
 * @example {
    "redirect_url": "https://github.com/login/oauth/authorize?client_id=abc",
    "state": "csrf-token-xyz"
}
 */
export type LinkSocialAccountResponse = {
    /**
     * URL to redirect the user's browser to the OAuth provider for linking
     */
    redirect_url: string;
    /**
     * Unique state token for CSRF protection, must be included in the callback
     */
    state: string;
};

