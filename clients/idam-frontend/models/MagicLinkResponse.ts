/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * @example {
    "magic_link_sent": true,
    "expires_in": 900,
    "message": "A magic link has been sent to your email"
}
 */
export type MagicLinkResponse = {
    /**
     * Whether magic link was sent
     * @example true
     */
    magic_link_sent: boolean;
    /**
     * Seconds until expiration
     * @example 900
     */
    expires_in?: number;
};

