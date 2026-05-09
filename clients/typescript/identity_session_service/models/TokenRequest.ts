/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type TokenRequest = {
    grant_type: 'refresh_token' | 'client_credentials' | 'urn:ietf:params:oauth:grant-type:token-exchange';
    refresh_token?: string;
    client_id?: string;
    client_secret?: string;
    /**
     * For token_exchange
     */
    subject_token?: string;
    subject_token_type?: string;
    audience?: string;
    scope?: string;
};

