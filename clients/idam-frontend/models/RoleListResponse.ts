/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { Role } from './Role';
/**
 * @example {
    "roles": [
        {
            "role_id": "550e8400-e29b-41d4-a716-446655440010",
            "name": "Project Manager",
            "description": "Can manage projects and view team members",
            "permissions": [
                "project:read",
                "project:write",
                "team:read"
            ],
            "created_at": "2024-01-16T12:00:00Z"
        },
        {
            "role_id": "550e8400-e29b-41d4-a716-446655440011",
            "name": "Viewer",
            "description": "Read-only access",
            "permissions": [
                "project:read"
            ],
            "created_at": "2024-01-10T00:00:00Z"
        }
    ],
    "total": 2,
    "page": 1,
    "limit": 20
}
 */
export type RoleListResponse = {
    items: Array<Role>;
    total: number;
    page: number;
    page_size: number;
};

