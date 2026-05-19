Feature: Get Effective Permissions (POST /authz/principals/effective)

  As an application or service
  I want to retrieve all roles, permissions, and attributes for a principal
  So that I can make authorization decisions based on their rights

  Background:
    Given the authz-core service is running
    And I have a valid Ed25519-signed JWT for tenant "6ba7b810-9dad-11d1-80b4-00c04fd430c8"

  # ─── Scenario Group 1: Successful retrieval ───

  Scenario: Get effective permissions for a user
    When I send a POST request to "/authz/principals/effective" with:
      """
      {
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "app_id": "33333333-8a2d-4c41-8b4b-ae43ce79a494"
      }
      """
    Then the response status is 200
    And the response body has field "user_id"
    And the response body has field "roles"
    And the response body has field "permissions"

  Scenario: Get effective permissions with inheritance
    When I send a POST request to "/authz/principals/effective" with:
      """
      {
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "app_id": "33333333-8a2d-4c41-8b4b-ae43ce79a494",
        "include_inherited": true
      }
      """
    Then the response status is 200
    And the response body has field "roles"
    And roles may include inherited entries

  # ─── Scenario Group 2: Required fields validation ───

  Scenario: Reject request missing required "user_id" field
    When I send a POST request to "/authz/principals/effective" with:
      """
      {
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "app_id": "33333333-8a2d-4c41-8b4b-ae43ce79a494"
      }
      """
    Then the response status is 400

  Scenario: Reject request missing required "tenant_id" field
    When I send a POST request to "/authz/principals/effective" with:
      """
      {
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "app_id": "33333333-8a2d-4c41-8b4b-ae43ce79a494"
      }
      """
    Then the response status is 400

  Scenario: Reject request missing required "app_id" field
    When I send a POST request to "/authz/principals/effective" with:
      """
      {
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
      }
      """
    Then the response status is 400

  # ─── Scenario Group 3: Response shape validation ───

  Scenario: Response contains "user_id" string
    When I send a POST request to "/authz/principals/effective" with:
      """
      {
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "app_id": "33333333-8a2d-4c41-8b4b-ae43ce79a494"
      }
      """
    Then the response body is valid JSON
    And the response body has a "user_id" field of type string

  Scenario: Response contains "roles" array
    When I send a POST request to "/authz/principals/effective" with:
      """
      {
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "app_id": "33333333-8a2d-4c41-8b4b-ae43ce79a494"
      }
      """
    Then the response body has a "roles" field of type array

  Scenario: Response contains "permissions" array
    When I send a POST request to "/authz/principals/effective" with:
      """
      {
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "app_id": "33333333-8a2d-4c41-8b4b-ae43ce79a494"
      }
      """
    Then the response body has a "permissions" field of type array

  # ─── Scenario Group 4: Optional fields ───

  Scenario: Accept request with optional "org_id" field
    When I send a POST request to "/authz/principals/effective" with:
      """
      {
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "app_id": "33333333-8a2d-4c41-8b4b-ae43ce79a494",
        "org_id": "22222222-8a2d-4c41-8b4b-ae43ce79a493"
      }
      """
    Then the response status is 200
    And the response body has field "roles"

  # ─── Scenario Group 5: Audit event emission ───

  Scenario: Effective permissions request emits audit event
    When I send a POST request to "/authz/principals/effective" with:
      """
      {
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "app_id": "33333333-8a2d-4c41-8b4b-ae43ce79a494"
      }
      """
    Then the response status is 200
    And an audit event is emitted with type "authorization"
