/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type DualOTPVerifyRequest = {
    /**
     * Email address to verify
     */
    email: string;
    /**
     * 6-digit email OTP code (omit if already verified)
     */
    email_code?: string | null;
    /**
     * Phone number to verify (E.164 format)
     */
    phone: string;
    /**
     * 6-digit phone OTP code (omit if already verified)
     */
    phone_code?: string | null;
    /**
     * Session ID from the dual-otp send step
     */
    session_id?: string;
};

