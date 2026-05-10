/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * @example {
    "id": "550e8400-e29b-41d4-a716-446655440001",
    "name": "Hauliage Web App",
    "slug": "hauliage-web",
    "org_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
    "created_at": "2024-01-01T00:00:00Z",
    "updated_at": "2024-06-15T12:00:00Z"
}
 */
export type Application = {
    id: string;
    /**
     * Application name
     */
    name: string;
    /**
     * Application URL-safe slug
     */
    slug: string;
    /**
     * Owning organisation ID
     */
    org_id?: string;
    created_at: string;
    updated_at?: string;
};

