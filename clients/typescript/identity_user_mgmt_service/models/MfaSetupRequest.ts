/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Request to set up TOTP MFA for a user.
 * Requires the user's current password for verification.
 *
 */
/**
 * Example usage:
 * ```typescript
 * const example: MfaSetupRequest = {
  "password": "secret123",
  "name": "Google Authenticator"
};
 * ```
 */

export type MfaSetupRequest = {
    /**
     * User's current password for verification
     */
    password: string;
    /**
     * Optional label for the TOTP factor (e.g. "Google Authenticator", "Work phone").
     * Shown in MFA management UI.
     *
     */
    name?: string | null;
};

