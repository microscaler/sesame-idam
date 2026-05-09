/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Returned when the login attempt requires a second factor.
 * The client must present the appropriate factor code before completing auth.
 *
 */
/**
 * Example usage:
 * ```typescript
 * const example: MfaRequiredResponse = {
  "session_id": "sess_abc123",
  "mfa_required": true,
  "mfa_type": "sms"
};
 * ```
 */

export type MfaRequiredResponse = {
    /**
     * Opaque session identifier for the auth flow. Must be included
     * in the subsequent MFA verification call.
     *
     */
    session_id?: string;
    mfa_required?: boolean;
    /**
     * The MFA factor the client must present next:
     * - totp: 6-digit TOTP code from authenticator app
     * - webauthn: Biometric or hardware key challenge
     * - sms: 4-6 digit SMS code
     * - email: 4-6 digit email code
     *
     */
    mfa_type?: 'totp' | 'webauthn' | 'sms' | 'email';
    /**
     * Optional challenge payload for WebAuthn verification.
     * Present when mfa_type is webauthn.
     *
     */
    mfa_challenge?: {
        challenge_id?: string;
        /**
         * Base64-encoded WebAuthn options
         */
        public_key_credential_request_options?: string;
    } | null;
};

