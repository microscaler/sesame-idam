/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * @example {
    "phone": "+1234567890",
    "code": "654321"
}
 */
export type PhoneOTPVerifyRequest = {
    /**
     * Phone number that received the OTP (E.164 format)
     */
    phone: string;
    /**
     * 6-digit SMS OTP code
     */
    code: string;
};

