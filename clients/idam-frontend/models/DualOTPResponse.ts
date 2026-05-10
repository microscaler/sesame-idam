/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * @example {
    "success": true,
    "email_sent": true,
    "phone_sent": true,
    "email_verified": false,
    "phone_verified": false,
    "both_verified": false,
    "message": "Verification codes have been sent to your email and phone"
}
 */
export type DualOTPResponse = {
    /**
     * @example true
     */
    success: boolean;
    /**
     * Whether OTP was sent to email
     */
    email_sent: boolean;
    /**
     * Whether OTP was sent to phone
     */
    phone_sent: boolean;
    /**
     * Whether email was previously verified
     */
    email_verified?: boolean;
    /**
     * Whether phone was previously verified
     */
    phone_verified?: boolean;
    /**
     * Whether both were already verified (auto-complete login)
     */
    both_verified?: boolean;
    /**
     * @example Verification codes have been sent to your email and phone
     */
    message?: string;
};

