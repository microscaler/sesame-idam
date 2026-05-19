Feature: Authorization Check (POST /authz/authorize)

  As an API consumer
  I want to check if a principal is allowed to perform an action on a resource
  So that I can enforce fine-grained access control

  Background:
    Given the authz-core service is running
    And I have a valid Ed25519-signed JWT for tenant "6ba7b810-9dad-11d1-80b4-00c04fd430c8"

  # ─── Scenario Group 1: Successful authorization ───

  Scenario: Allow read action on a resource
    When I send a POST request to "/authz/authorize" with:
      """
      {
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "action": "read",
        "resource": "accounting:invoices",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
      }
      """
    Then the response status is 200
    And the response body has field "allowed" set to true

  # ─── Scenario Group 2: Required fields validation ───

  Scenario: Reject request missing required "action" field
    When I send a POST request to "/authz/authorize" with:
      """
      {
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "resource": "accounting:invoices",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
      }
      """
    Then the response status is 400

  Scenario: Reject request missing required "resource" field
    When I send a POST request to "/authz/authorize" with:
      """
      {
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "action": "read",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
      }
      """
    Then the response status is 400

  # ─── Scenario Group 3: Optional fields ───

  Scenario: Accept request with optional "org_id" field
    When I send a POST request to "/authz/authorize" with:
      """
      {
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "action": "write",
        "resource": "accounting:invoices",
        "org_id": "22222222-8a2d-4c41-8b4b-ae43ce79a493",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
      }
      """
    Then the response status is 200
    And the response body has field "allowed"

  Scenario: Accept request with optional "app_id" field
    When I send a POST request to "/authz/authorize" with:
      """
      {
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "action": "delete",
        "resource": "accounting:invoices",
        "app_id": "33333333-8a2d-4c41-8b4b-ae43ce79a494",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
      }
      """
    Then the response status is 200
    And the response body has field "allowed"

  # ─── Scenario Group 4: Response shape validation ───

  Scenario: Response contains "allowed" boolean
    When I send a POST request to "/authz/authorize" with:
      """
      {
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "action": "read",
        "resource": "accounting:invoices",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
      }
      """
    Then the response body is valid JSON
    And the response body has a "allowed" field of type boolean

  Scenario: Response may contain optional "permissions_used" array
    When I send a POST request to "/authz/authorize" with:
      """
      {
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "action": "read",
        "resource": "accounting:invoices",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
      }
      """
    Then the response body may contain a "permissions_used" field of type array

  Scenario: Response may contain optional "reason" string
    When I send a POST request to "/authz/authorize" with:
      """
      {
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "action": "read",
        "resource": "accounting:invoices",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
      }
      """
    Then the response body may contain a "reason" field of type string

  # ─── Scenario Group 5: Audit event emission ───

  Scenario: Authorization request emits audit event
    When I send a POST request to "/authz/authorize" with:
      """
      {
        "user_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "action": "read",
        "resource": "accounting:invoices",
        "tenant_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
      }
      """
    Then the response status is 200
    And an audit event is emitted with type "authorization"
