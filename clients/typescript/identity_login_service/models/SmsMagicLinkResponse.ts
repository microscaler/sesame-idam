/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Property examples:
 *  * - `magic_link_sent`: `true`
 * - `expires_in`: `900`
 */

export type SmsMagicLinkResponse = {
    /**
     * Whether SMS magic link was sent
     */
    magic_link_sent: boolean;
    /**
     * Seconds until expiration
     */
    expires_in?: number;
};

