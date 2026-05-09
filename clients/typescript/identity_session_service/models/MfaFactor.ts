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
    factor_type?: MfaFactor.factor_type;
    /**
     * Whether this is the primary MFA factor
     */
    is_primary?: boolean;
    /**
     * Unix timestamp of creation
     */
    created_at?: number;
};
export namespace MfaFactor {
    /**
     * The type of MFA factor (totp, sms, email, hardware_key)
     */
    export enum factor_type {
        TOTP = 'totp',
        SMS = 'sms',
        EMAIL = 'email',
        HARDWARE_KEY = 'hardware_key',
    }
}

