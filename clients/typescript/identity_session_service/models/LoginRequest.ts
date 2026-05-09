/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Authenticate with either email or phone_number.
 * Platform admins can optionally specify organization_id to scope the login.
 *
 */
export type LoginRequest = {
    /**
     * Primary email address
     */
    email: string;
    /**
     * Alternative login method using phone number (E.164 format).
     * If provided, email is ignored. Phone OTP flow returns MFA_REQUIRED.
     *
     */
    phone_number?: string;
    /**
     * User's password (only required when logging in with email, not with phone OTP)
     */
    password: string;
    /**
     * Optional organisation hint for multi-tenant orgs
     */
    organization_id?: string | null;
    /**
     * Tenant for multi-tenant orgs
     */
    tenant_id?: string | null;
};

