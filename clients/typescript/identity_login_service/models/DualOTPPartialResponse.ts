/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Property examples:
 *  * - `success`: `true`
 * - `message`: `"Email verified. Please verify your phone to complete login.`
 */

export type DualOTPPartialResponse = {
    success: boolean;
    /**
     * Current email verification status
     */
    email_verified: boolean;
    /**
     * Current phone verification status
     */
    phone_verified: boolean;
    /**
     * Always false — both codes still needed
     */
    both_verified: boolean;
    message?: string;
};

