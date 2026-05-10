/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * @example {
    "mfa_required": true,
    "challenge_type": "totp",
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "expires_in": 300
}
 */
export type MfaRequiredResponse = {
    /**
     * @example true
     */
    mfa_required: boolean;
    /**
     * Type of MFA challenge to complete
     */
    challenge_type: MfaRequiredResponse.challenge_type;
    /**
     * Session identifier for completing the MFA step
     */
    session_id?: string;
    /**
     * Seconds until the MFA challenge expires
     */
    expires_in?: number;
};
export namespace MfaRequiredResponse {
    /**
     * Type of MFA challenge to complete
     */
    export enum challenge_type {
        TOTP = 'totp',
        SMS = 'sms',
        EMAIL = 'email',
        WEBAUTHN = 'webauthn',
    }
}

