/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * @example {
    "email": "alice@example.com",
    "code": "123456"
}
 */
export type EmailOTPVerifyRequest = {
    /**
     * Email address that received the OTP
     */
    email: string;
    /**
     * 6-digit OTP code sent to email
     */
    code: string;
};

