/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type MigratePasswordRequest = {
    users: Array<{
        email: string;
        /**
         * bcrypt hash ($2a4$...)
         */
        hash: string;
        /**
         * Original salt if different from hash
         */
        salt: string;
    }>;
};

