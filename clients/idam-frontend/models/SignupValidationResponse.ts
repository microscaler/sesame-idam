/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * @example {
    "allowed": true,
    "reason": null,
    "email_exists": false,
    "phone_exists": false,
    "suggested_username": "alice_new"
}
 */
export type SignupValidationResponse = {
    /**
     * Whether the email/phone is allowed to register
     */
    allowed: boolean;
    /**
     * List of rejection reasons (empty if allowed)
     */
    reasons?: Array<string>;
    /**
     * Whether the user will need MFA after registration
     */
    requires_mfa?: boolean;
};

