/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Returns the TOTP provisioning URI that can be rendered as a QR code.
 * The user must scan the QR code and enter the resulting code to complete setup.
 *
 * @example {
    "mfa_required": true,
    "secret": "JBSWY3DPEHPK3PXP",
    "qr_code": "data:image/png;base64,example",
    "backup_codes": [
        "12345678",
        "87654321",
        "11111111",
        "22222222",
        "33333333"
    ]
}
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

