/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * An enabled MFA factor for the user
 */
export type MfaFactor = {
    /**
     * The type of MFA factor (totp, sms, email, hardware_key)
     */
    factor_type?: 'totp' | 'sms' | 'email' | 'hardware_key';
    /**
     * Whether this is the primary MFA factor
     */
    is_primary?: boolean;
    /**
     * Unix timestamp of creation
     */
    created_at?: number;
};

