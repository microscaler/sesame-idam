/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type RegisterRequest = {
    /**
     * Primary email address (must be unique)
     */
    email: string;
    /**
     * User's password (minimum 8 characters)
     */
    password: string;
    /**
     * User's first name
     */
    first_name?: string | null;
    /**
     * User's last name
     */
    last_name?: string | null;
    /**
     * Optional human-identifiable username
     */
    username?: string | null;
    /**
     * Phone number in E.164 format (for dual verification flow)
     */
    phone?: string | null;
};

