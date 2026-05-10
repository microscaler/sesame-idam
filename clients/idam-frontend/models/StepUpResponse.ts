/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * @example {
    "access_token": "eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiIxMjMiLCJsZXZlbCI6ImhpZ2gifQ.sig",
    "token_type": "Bearer",
    "expires_in": 300,
    "refresh_token": "bmV3LXJlZnJlc2gtdG9rZW4tc3RlcC11cA",
    "refresh_token_expires_in": 3600,
    "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
    "email": "alice@example.com",
    "email_verified": true,
    "phone_verified": false,
    "mfa_required": false,
    "id_token": null,
    "scope": "openid profile"
}
 */
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

