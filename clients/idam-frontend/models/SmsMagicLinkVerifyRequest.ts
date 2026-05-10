/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * @example {
    "token": "sms-magic-token-xyz"
}
 */
export type SmsMagicLinkVerifyRequest = {
    /**
     * Phone number that received the SMS
     */
    phone: string;
    /**
     * Magic link token from the SMS URL
     */
    token: string;
};

