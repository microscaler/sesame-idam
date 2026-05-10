/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Request body for logout operation. Either the refresh_token in the body OR the Bearer token in the Authorization header can be used to identify the session to revoke. If both are provided, the refresh_token is preferred.
 * @example {
    "refresh_token": "cmVmcmVzaC10b2tlbi1hbGljZS1zZXNzaW9u"
}
 */
export type LogoutRequest = {
    /**
     * The refresh token to revoke. Required only if no Authorization header is present. If the session is identified via the Bearer token in the Authorization header, this field is optional.
     */
    refresh_token?: string;
};

