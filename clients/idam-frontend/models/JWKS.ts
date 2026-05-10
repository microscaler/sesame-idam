/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type JWKS = {
    /**
     * JSON Web Key Set containing the signing keys for JWT verification.
     * Keys may be rotated; consumers should cache but periodically re-fetch.
     *
     */
    keys: Array<{
        /**
         * Key type (e.g. "RSA")
         */
        kty: string;
        /**
         * Key ID — unique identifier for this key (used by JWT `kid` header)
         */
        kid: string;
        /**
         * Intended use of the public key (sig = signature)
         */
        use: string;
        /**
         * Intended algorithm for using the key (e.g. "RS256")
         */
        alg: string;
        /**
         * RSA modulus (base64url-encoded)
         */
        'n': string;
        /**
         * RSA exponent (base64url-encoded)
         */
        'e': string;
        /**
         * X.509 certificate chain (base64url-encoded DER).
         * Enables certificate pinning and trust anchor validation.
         *
         */
        x5c: Array<string>;
        /**
         * X.509 certificate thumbprint (base64url-encoded SHA-1)
         */
        x5t?: string | null;
    }>;
};

