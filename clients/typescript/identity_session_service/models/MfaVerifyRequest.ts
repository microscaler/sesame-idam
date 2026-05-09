/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Verifies an MFA code. Can be used for:
 * 1. Login step 2 (after MFA_REQUIRED response, includes session_id)
 * 2. MFA setup step 2 (after MFA setup generated QR code)
 *
 */
export type MfaVerifyRequest = {
    /**
     * The MFA verification code (6-digit TOTP, 4-6 digit SMS, etc.)
     */
    code: string;
    /**
     * Session ID from MFA_REQUIRED login response. Required for step 2 of login.
     *
     */
    session_id?: string | null;
    /**
     * WebAuthn challenge ID from MFA_REQUIRED response (for webauthn factor type).
     *
     */
    challenge_id?: string | null;
};

