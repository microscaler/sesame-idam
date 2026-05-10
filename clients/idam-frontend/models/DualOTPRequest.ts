/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * @example {
    "email": "alice@example.com",
    "phone": "+1234567890"
}
 */
export type DualOTPRequest = {
    /**
     * Email address to send OTP to
     */
    email: string;
    /**
     * Phone number in E.164 format (e.g. +14155551234)
     */
    phone: string;
    /**
     * Send welcome email after successful verification
     */
    send_welcome_email?: boolean;
};

