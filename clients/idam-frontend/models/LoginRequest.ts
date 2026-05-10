/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * @example {
    "email": "alice@example.com",
    "password": "SecureP@ss123!",
    "organization_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492"
}
 */
export type LoginRequest = {
    /**
     * Primary email address
     */
    email: string;
    /**
     * User's password
     */
    password: string;
    /**
     * Optional org hint for multi-tenant login
     */
    organization_id?: string | null;
};

