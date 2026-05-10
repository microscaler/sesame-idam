/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * @example {
    "success": true,
    "email_verified": true,
    "phone_verified": false,
    "both_verified": false,
    "message": "Email verified. Please verify your phone to complete login."
}
 */
export type DualOTPPartialResponse = {
    /**
     * @example true
     */
    success: boolean;
    /**
     * Current email verification status
     */
    email_verified: boolean;
    /**
     * Current phone verification status
     */
    phone_verified: boolean;
    /**
     * Always false — both codes still needed
     */
    both_verified: boolean;
    /**
     * @example Email verified. Please verify your phone to complete login.
     */
    message?: string;
};

