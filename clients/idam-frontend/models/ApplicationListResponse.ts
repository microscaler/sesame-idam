/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { Application } from './Application';
/**
 * @example {
    "items": [
        {
            "id": "550e8400-e29b-41d4-a716-446655440001",
            "name": "Hauliage Web App",
            "slug": "hauliage-web",
            "org_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-06-15T12:00:00Z"
        }
    ],
    "total": 1,
    "page": 1,
    "page_size": 20
}
 */
export type ApplicationListResponse = {
    items: Array<Application>;
    total: number;
    page: number;
    page_size: number;
};

