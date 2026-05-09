/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type TokenIssuanceRequest = {
    /**
     * Target user for token issuance
     */
    user_id: string;
    /**
     * Token scope level
     */
    scope: 'full' | 'read' | 'write';
    /**
     * Token lifetime in seconds (default: 1 hour)
     */
    expires_in?: number;
};

