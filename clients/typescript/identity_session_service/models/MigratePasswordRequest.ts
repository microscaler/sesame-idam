/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type MigratePasswordRequest = {
    migrations: Array<{
        email: string;
        /**
         * bcrypt hash format: $2a$14$...
         */
        hash: string;
        salt: string;
    }>;
};

