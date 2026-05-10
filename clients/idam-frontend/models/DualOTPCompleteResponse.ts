/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { TokenResponse } from './TokenResponse';
/**
 * @example {
    "access_token": "eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiIxMjMiLCJlbWFpbCI6ImFsaWNlQGV4cC5jb20ifQ.sig",
    "token_type": "Bearer",
    "expires_in": 900,
    "refresh_token": "cmVmcmVzaC10b2tlbi1kdWFsLW90cA",
    "refresh_token_expires_in": 2592000,
    "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
    "email": "alice@example.com",
    "email_verified": true,
    "phone_verified": true,
    "mfa_required": false,
    "id_token": null,
    "scope": "openid"
}
 */
export type DualOTPCompleteResponse = (TokenResponse & {
    /**
     * Whether email was just verified in this operation
     */
    newly_verified_email?: boolean;
    /**
     * Whether phone was just verified in this operation
     */
    newly_verified_phone?: boolean;
});

