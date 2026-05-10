/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Verify a phone number via SMS OTP code.
 *
 * @example {
    "phone": "+1234567890",
    "code": "654321"
}
 */
export type PhoneVerificationRequest = {
    /**
     * OTP code received via SMS
     */
    code?: string;
    /**
     * Phone number to verify (E.164 format)
     */
    phone_number?: string;
};

