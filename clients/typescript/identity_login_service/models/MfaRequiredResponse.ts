/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Property examples:
 *  * - `mfa_required`: `true`
 */

export type MfaRequiredResponse = {
    mfa_required: boolean;
    /**
     * Type of MFA challenge to complete
     */
    challenge_type: 'totp' | 'sms' | 'email' | 'webauthn';
    /**
     * Session identifier for completing the MFA step
     */
    session_id?: string;
    /**
     * Seconds until the MFA challenge expires
     */
    expires_in?: number;
};

