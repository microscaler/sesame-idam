/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type StepUpResponse = {
    /**
     * Whether step-up verification succeeded
     */
    verified: boolean;
    /**
     * MFA method used for verification
     */
    mfa_method?: string;
    /**
     * Session with elevated trust level
     */
    session_id?: string;
};

