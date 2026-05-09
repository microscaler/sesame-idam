/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
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

