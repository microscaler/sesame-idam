/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * @example {
    "role_id": "550e8400-e29b-41d4-a716-446655440010",
    "name": "Project Manager",
    "description": "Can manage projects and view team members",
    "permissions": [
        "project:read",
        "project:write",
        "team:read"
    ],
    "created_at": "2024-01-16T12:00:00Z"
}
 */
export type Role = {
    id: string;
    /**
     * Role name
     */
    name: string;
    /**
     * Role description
     */
    description?: string;
    /**
     * Owning application ID
     */
    application_id: string;
    created_at: string;
    updated_at?: string;
};

