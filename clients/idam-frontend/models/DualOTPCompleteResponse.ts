/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { TokenResponse } from './TokenResponse';
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

