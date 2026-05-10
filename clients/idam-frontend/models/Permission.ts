/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * @example {
    "id": "550e8400-e29b-41d4-a716-446655440010",
    "name": "trucking:routes:manage",
    "description": "Create and manage truck routes",
    "application_id": "550e8400-e29b-41d4-a716-446655440001",
    "created_at": "2024-01-01T00:00:00Z",
    "updated_at": "2024-03-15T00:00:00Z"
}
 */
export type Permission = {
    id: string;
    /**
     * Permission name (e.g. "read:docs")
     */
    name: string;
    /**
     * Permission description
     */
    description?: string;
    /**
     * Owning application ID
     */
    application_id: string;
    created_at: string;
    updated_at?: string;
};

