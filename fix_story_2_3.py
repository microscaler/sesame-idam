#!/usr/bin/env python3
"""
Story 2.3: Replace PII Fields with References
OpenAPI spec changes:
1. Remove PII fields from TokenResponse: email, email_verified, phone_verified
2. Add entitlements_ref and entitlements_hash to TokenResponse
3. Add GET /users/me endpoint with UserProfileResponse schema
4. Update response examples to remove PII
5. Add UserManagement tag
"""

import yaml
import sys
from copy import deepcopy

SPEC_PATH = "/home/casibbald/Workspace/microscaler/seasame-idam/openapi/idam/identity-login-service/openapi.yaml"

PII_FIELDS = ["email", "email_verified", "phone_number", "phone_verified", "first_name", "last_name", "name", "preferred_username"]
TOKEN_RESPONSE_PII = ["email", "email_verified", "phone_verified"]

def load_spec():
    with open(SPEC_PATH) as f:
        return yaml.safe_load(f)

def save_spec(data):
    with open(SPEC_PATH, 'w') as f:
        yaml.dump(data, f, sort_keys=False, width=120, default_flow_style=False)

def remove_pii_from_schema(schema):
    """Remove PII fields from a schema's properties."""
    if not schema or schema.get("type") != "object":
        return
    props = schema.get("properties", {})
    if not props:
        return
    
    # Remove PII fields
    for pii in TOKEN_RESPONSE_PII:
        if pii in props:
            del props[pii]
            print(f"  Removed PII field '{pii}' from schema")
    
    # Update required if needed
    required = schema.get("required", [])
    schema["required"] = [r for r in required if r not in TOKEN_RESPONSE_PII]
    if "required" not in schema or not schema["required"]:
        if "required" in schema:
            del schema["required"]

def update_token_response(data):
    """Update TokenResponse schema to remove PII and add entitlements fields."""
    print("\n=== Updating TokenResponse schema ===")
    
    schemas = data["components"]["schemas"]
    token_resp = schemas.get("TokenResponse", {})
    
    # Remove PII fields from properties
    props = token_resp.get("properties", {})
    
    for pii in TOKEN_RESPONSE_PII:
        if pii in props:
            del props[pii]
            print(f"  Removed PII field '{pii}' from TokenResponse")
    
    # Remove from required
    required = token_resp.get("required", [])
    token_resp["required"] = [r for r in required if r not in TOKEN_RESPONSE_PII]
    
    # Add entitlements fields
    props["entitlements_ref"] = {
        "type": "string",
        "description": "Deterministic reference to the user's entitlements snapshot (format: ent_<uuid_v5>)",
        "example": "ent_a1b2c3d4-e5f6-7890-abcd-ef1234567890"
    }
    props["entitlements_hash"] = {
        "type": "string",
        "description": "SHA-256 hash of the canonical JSON representation of the entitlements snapshot (format: sha256:<hex>)",
        "example": "sha256:a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"
    }
    props["roles"] = {
        "type": "array",
        "items": {"type": "string"},
        "description": "User's roles within the organization",
        "example": ["admin", "billing-viewer"]
    }
    props["permissions"] = {
        "type": "array",
        "items": {"type": "string"},
        "description": "Bounded set of permissions from the entitlements snapshot",
        "example": ["org:admin", "billing:read"]
    }
    
    token_resp["properties"] = props
    
    # Add EntitlementsSnapshot schema
    print("\n=== Adding EntitlementsSnapshot schema ===")
    schemas["EntitlementsSnapshot"] = {
        "type": "object",
        "required": ["version", "permissions", "roles", "tenant"],
        "properties": {
            "version": {
                "type": "integer",
                "description": "Entitlements version, bumped on permission changes"
            },
            "permissions": {
                "type": "array",
                "items": {"type": "string"},
                "description": "Full list of permissions for this user/organization combination"
            },
            "roles": {
                "type": "array",
                "items": {"type": "string"},
                "description": "Roles assigned to this user"
            },
            "tenant": {
                "type": "string",
                "format": "uuid",
                "description": "Tenant identifier for multi-tenant isolation"
            },
            "hash": {
                "type": "string",
                "description": "SHA-256 hash of the canonical JSON of this snapshot"
            }
        }
    }
    
    # Add UserProfileResponse schema
    print("\n=== Adding UserProfileResponse schema ===")
    schemas["UserProfileResponse"] = {
        "type": "object",
        "required": ["user_id", "email", "email_verified"],
        "properties": {
            "user_id": {
                "type": "string",
                "format": "uuid",
                "description": "UUID of the user"
            },
            "email": {
                "type": "string",
                "format": "email",
                "description": "User's email address"
            },
            "email_verified": {
                "type": "boolean",
                "description": "Whether the email has been verified"
            },
            "phone_number": {
                "type": "string",
                "pattern": "^\\+[1-9]\\d{1,14}$",
                "nullable": True,
                "description": "Phone number in E.164 format"
            },
            "phone_verified": {
                "type": "boolean",
                "description": "Whether the phone has been verified"
            },
            "first_name": {
                "type": "string",
                "nullable": True,
                "description": "User's first name"
            },
            "last_name": {
                "type": "string",
                "nullable": True,
                "description": "User's last name"
            },
            "name": {
                "type": "string",
                "nullable": True,
                "description": "Full display name"
            },
            "preferred_username": {
                "type": "string",
                "nullable": True,
                "description": "Preferred username"
            }
        }
    }
    
    return token_resp

def add_users_me_endpoint(data):
    """Add GET /api/v1/identity/users/me endpoint."""
    print("\n=== Adding GET /users/me endpoint ===")
    
    paths = data["paths"]
    
    # Add to tags
    if "UserManagement" not in [t["name"] for t in data.get("tags", [])]:
        data["tags"].append({
            "name": "UserManagement",
            "description": "User profile management and PII retrieval endpoints"
        })
    
    paths["/auth/users/me"] = {
        "get": {
            "tags": ["UserManagement"],
            "summary": "Get current user profile with PII",
            "description": "Returns the authenticated user's full profile including PII fields.\n\nThis is the designated endpoint for fetching PII that was removed from JWT tokens in Story 2.3.\n\nRequires a valid Bearer token with an active session.",
            "operationId": "get_user_profile",
            "responses": {
                "200": {
                    "description": "User profile retrieved successfully",
                    "content": {
                        "application/json": {
                            "schema": {"$ref": "#/components/schemas/UserProfileResponse"},
                            "example": {
                                "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
                                "email": "alice@example.com",
                                "email_verified": True,
                                "phone_number": "+14155551234",
                                "phone_verified": False,
                                "first_name": "Alice",
                                "last_name": "Smith",
                                "name": "Alice Smith",
                                "preferred_username": "asmith"
                            }
                        }
                    }
                },
                "401": {
                    "description": "Unauthorized — invalid, expired, or missing credentials",
                    "content": {
                        "application/json": {
                            "schema": {"$ref": "#/components/schemas/ErrorResponse"},
                            "example": {
                                "error": "unauthorized",
                                "error_description": "Authentication required"
                            }
                        }
                    }
                },
                "500": {
                    "description": "Internal server error",
                    "content": {
                        "application/json": {
                            "schema": {"$ref": "#/components/schemas/ErrorResponse"},
                            "example": {
                                "error": "internal_error",
                                "error_description": "An unexpected error occurred"
                            }
                        }
                    }
                }
            },
            "security": [{"BearerAuth": []}]
        }
    }

def update_all_token_responses(data):
    """Update all schemas that reference TokenResponse."""
    print("\n=== Updating all TokenResponse-inheriting schemas ===")
    
    schemas = data["components"]["schemas"]
    
    # Schemas that inherit from TokenResponse via allOf
    for name, schema in schemas.items():
        all_of = schema.get("allOf", [])
        for ref_obj in all_of:
            if "$ref" in ref_obj and ref_obj["$ref"] == "#/components/schemas/TokenResponse":
                print(f"  Processing {name} (allOf -> TokenResponse)")
                # Add properties that override/remove PII
                props = schema.setdefault("properties", {})
                for pii in TOKEN_RESPONSE_PII:
                    if pii in props:
                        del props[pii]
                        print(f"    Removed '{pii}' from {name}")
    
    # Also handle schemas with direct PII properties
    for name, schema in schemas.items():
        if schema.get("type") == "object":
            props = schema.get("properties", {})
            for pii in TOKEN_RESPONSE_PII:
                if pii in props:
                    # Check if it's in a TokenResponse-like schema
                    if "TokenResponse" in name or name in ["DualOTPCompleteResponse", "SocialLoginResponse", "SocialCallbackResponse"]:
                        del props[pii]
                        print(f"  Removed '{pii}' from {name}")

def update_response_examples(data):
    """Remove PII from all response examples."""
    print("\n=== Removing PII from response examples ===")
    
    paths = data.get("paths", {})
    for path, methods in paths.items():
        for method, op in methods.items():
            if method == "get":
                continue
            responses = op.get("responses", {})
            for status_code, resp in responses.items():
                content = resp.get("content", {})
                for media_type, media in content.items():
                    example = media.get("example")
                    if isinstance(example, dict):
                        for pii in TOKEN_RESPONSE_PII:
                            if pii in example:
                                del example[pii]
                                print(f"  Removed '{pii}' from example at {path}/{method}/{status_code}")
                    
                    # Also check nested examples
                    examples = media.get("examples", {})
                    if isinstance(examples, dict):
                        for ex_name, ex_val in examples.items():
                            ex_value = ex_val.get("value") if isinstance(ex_val, dict) else ex_val
                            if isinstance(ex_value, dict):
                                for pii in TOKEN_RESPONSE_PII:
                                    if pii in ex_value:
                                        del ex_value[pii]
                                        print(f"  Removed '{pii}' from nested example at {path}/{method}/{status_code}/{ex_name}")

def update_request_body_examples(data):
    """Remove PII fields from request body examples that are in response examples."""
    print("\n=== Checking request body examples ===")
    # RegisterRequest can still have first_name, last_name - those are inputs
    # We only remove PII from OUTPUT responses, not inputs
    pass

def main():
    print("Loading OpenAPI spec...")
    data = load_spec()
    
    print("Updating TokenResponse schema...")
    update_token_response(data)
    
    print("\nAdding /users/me endpoint...")
    add_users_me_endpoint(data)
    
    print("\nUpdating all TokenResponse-inheriting schemas...")
    update_all_token_responses(data)
    
    print("\nRemoving PII from response examples...")
    update_response_examples(data)
    
    print("\nSaving spec...")
    save_spec(data)
    
    print("\n=== Done! ===")

if __name__ == "__main__":
    main()
