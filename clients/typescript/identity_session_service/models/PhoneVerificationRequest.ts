/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Verify a phone number via SMS OTP code.
 *
 */
/**
 * Example usage:
 * ```typescript
 * const example: PhoneVerificationRequest = {
  "phone_number": "+14155551234",
  "code": "123456"
};
 * ```
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

