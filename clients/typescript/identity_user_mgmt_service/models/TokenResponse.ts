/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type TokenResponse = {
    access_token: string;
    refresh_token?: string | null;
    id_token?: string | null;
    token_type: TokenResponse.token_type;
    expires_in: number;
    scope?: string | null;
};
export namespace TokenResponse {
    export enum token_type {
        BEARER = 'Bearer',
    }
}

