/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * @example {
    "keys": [
        {
            "kty": "RSA",
            "kid": "default",
            "use": "sig",
            "alg": "RS256",
            "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbApbghMi8nF-_S0AI4-eJad0a30iV3-V2XN0b4g9S1_Hk09HM5y1nVAGTovsJ34vcEe",
            "e": "AQAB"
        }
    ]
}
 */
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

