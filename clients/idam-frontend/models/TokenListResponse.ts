/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * @example {
    "oauth_tokens": [
        {
            "id": "550e8400-e29b-41d4-a716-446655440001",
            "provider": "github",
            "provider_user_id": "12345",
            "scope": "repo,user",
            "created_at": "2024-01-10T00:00:00Z",
            "expires_at": null
        },
        {
            "id": "550e8400-e29b-41d4-a716-446655440002",
            "provider": "google",
            "provider_user_id": "67890",
            "scope": "email,profile",
            "created_at": "2024-01-12T00:00:00Z",
            "expires_at": "2024-07-12T00:00:00Z"
        }
    ]
}
 */
export type TokenListResponse = {
    tokens?: Array<{
        provider?: string;
        created_at?: number;
        expires_at?: number | null;
        scopes?: Array<string>;
    }>;
};

