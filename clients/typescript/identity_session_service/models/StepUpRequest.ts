/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type StepUpRequest = {
    /**
     * User ID performing the sensitive action
     */
    user_id: string;
    /**
     * The sensitive action requiring re-authentication
     */
    action: 'delete_account' | 'change_email' | 'change_password' | 'delete_org';
    /**
     * Current session identifier
     */
    session_id: string;
    /**
     * Preferred MFA method for re-authentication
     */
    mfa_method?: 'totp' | 'email' | 'phone';
};

