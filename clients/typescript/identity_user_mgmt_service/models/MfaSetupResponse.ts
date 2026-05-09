/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Returns the TOTP provisioning URI that can be rendered as a QR code.
 * The user must scan the QR code and enter the resulting code to complete setup.
 *
 */
/**
 * Example usage:
 * ```typescript
 * const example: MfaSetupResponse = {
  "provisioning_uri": "otpauth://totp/Sesame:test@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Sesame",
  "secret": "JBSWY3DPEHPK3PXP",
  "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d"
};
 * ```
 */

export type MfaSetupResponse = {
    /**
     * TOTP provisioning URI in the format:
     * `otpauth://totp/{issuer}:{user_id}?secret={secret}&issuer={issuer}`
     * This can be rendered as a QR code for the user to scan.
     *
     */
    provisioning_uri?: string;
    /**
     * Base32-encoded TOTP secret (also available in the provisioning URI).
     * Shown as a fallback if the QR code cannot be scanned.
     *
     */
    secret?: string;
    /**
     * The user ID for whom MFA is being set up
     */
    user_id?: string;
};

