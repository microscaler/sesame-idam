/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * @example {
    "challenge_id": "550e8400-e29b-41d4-a716-446655440000",
    "answer": "123456"
}
 */
export type StepUpRequest = {
    /**
     * User ID performing the sensitive action
     */
    user_id: string;
    /**
     * The sensitive action requiring re-authentication
     */
    action: StepUpRequest.action;
    /**
     * Current session identifier
     */
    session_id: string;
    /**
     * Preferred MFA method for re-authentication
     */
    mfa_method?: StepUpRequest.mfa_method;
};
export namespace StepUpRequest {
    /**
     * The sensitive action requiring re-authentication
     */
    export enum action {
        DELETE_ACCOUNT = 'delete_account',
        CHANGE_EMAIL = 'change_email',
        CHANGE_PASSWORD = 'change_password',
        DELETE_ORG = 'delete_org',
    }
    /**
     * Preferred MFA method for re-authentication
     */
    export enum mfa_method {
        TOTP = 'totp',
        EMAIL = 'email',
        PHONE = 'phone',
    }
}

