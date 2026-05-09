/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { TokenResponse } from './TokenResponse';
export type DualOTPCompleteResponse = (TokenResponse & {
    /**
     * Email verification status after this operation
     */
    email_verified?: boolean;
    /**
     * Phone verification status after this operation
     */
    phone_verified?: boolean;
    /**
     * Whether email was just verified in this operation
     */
    newly_verified_email?: boolean;
    /**
     * Whether phone was just verified in this operation
     */
    newly_verified_phone?: boolean;
});

