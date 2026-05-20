Feature: Delete Retention Policy
  As an admin or authorized user
  I want to delete audit log retention policies
  So that I can manage policy lifecycle

  Scenario: Delete policy with valid ID
    Given a valid tenant with policy "policy-123"
    When I request DELETE /authz/audit/events/retention/policy-123
    Then the response has an "error" field of type string

  Scenario: Delete with UUID format ID
    Given a valid tenant with policy "550e8400-e29b-41d4-a716-446655440000"
    When I request DELETE /authz/audit/events/retention/{id}
    Then the response has an "error" field of type string

  Scenario: Response "error_description" is optional
    Given a valid tenant
    When I request DELETE /authz/audit/events/retention/policy-123
    Then the response "error_description" is a string or null

  Scenario: Reject request missing required "id" field
    Given an invalid request without "id"
    Then deserialization fails with missing required field error

  Scenario: Reject request missing "X-Tenant-ID" header
    Given an invalid request without "X-Tenant-ID" header
    Then deserialization fails with missing required field error
