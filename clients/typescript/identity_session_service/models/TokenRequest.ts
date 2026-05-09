/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type TokenRequest = {
    grant_type: TokenRequest.grant_type;
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
export namespace TokenRequest {
    export enum grant_type {
        REFRESH_TOKEN = 'refresh_token',
        CLIENT_CREDENTIALS = 'client_credentials',
        URN_IETF_PARAMS_OAUTH_GRANT_TYPE_TOKEN_EXCHANGE = 'urn:ietf:params:oauth:grant-type:token-exchange',
    }
}

