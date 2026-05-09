/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type PendingEmailVerificationResponse = {
    /**
     * UUID of the newly created user
     */
    user_id: string;
    /**
     * Email address (unverified)
     */
    email: string;
    /**
     * Always false — user must confirm via email link
     */
    email_verified: boolean;
    message?: string;
};

