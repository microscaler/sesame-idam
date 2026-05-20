Feature: List Retention Policies
  As an admin or authorized user
  I want to list all audit log retention policies
  So that I can review existing policies

  Scenario: List returns empty when no policies
    Given a valid tenant
    And no retention policies exist
    When I request GET /authz/audit/events/retention
    Then the response is an empty array
    And the response serializes as a JSON array

  Scenario: Response is a valid JSON array
    Given a valid tenant
    When I request GET /authz/audit/events/retention
    Then the response body is a valid JSON array
    And each element has expected policy fields

  Scenario: Reject request missing "X-Tenant-ID" header
    Given an invalid request without "X-Tenant-ID" header
    Then deserialization fails with missing required field error
