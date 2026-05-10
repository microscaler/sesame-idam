/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type ResetPasswordRequest = {
    /**
     * Password reset token from the reset email
     */
    token: string;
    /**
     * New password (minimum 8 characters)
     */
    new_password: string;
};

