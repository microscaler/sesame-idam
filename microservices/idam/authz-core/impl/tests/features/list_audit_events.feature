Feature: List Audit Events
  As an admin or authorized user
  I want to list audit events with pagination and filtering
  So that I can review the audit trail

  Scenario: List returns empty when no data
    Given a valid tenant
    And no audit events exist
    When I request GET /authz/audit/events
    Then the response contains 0 items
    And the response has total_count 0
    And the response has an "items" field of type array
    And the response has a "total" field of type integer
    And the response has a "limit" field of type integer
    And the response has an "offset" field of type integer

  Scenario: List with pagination parameters
    Given a valid tenant
    And audit events exist in the database
    When I request GET /authz/audit/events with page=1 and limit=10
    Then the response contains the requested page
    And the response has "limit" field set to 10

  Scenario: List with event_type filter
    Given a valid tenant
    And audit events exist
    When I request GET /authz/audit/events with event_type=authentication
    Then the response returns matching events

  Scenario: List with severity filter
    Given a valid tenant
    And audit events exist
    When I request GET /authz/audit/events with severity=error
    Then the response returns matching events

  Scenario: List with time range filter
    Given a valid tenant
    And audit events exist
    When I request GET /authz/audit/events with start_time and end_time
    Then the response returns events within the range

  Scenario: Response schema validation
    Given a valid tenant
    When I request GET /authz/audit/events
    Then the response body has "items" field
    And the response body has "total" field
    And the response body has "limit" field
    And the response body has "offset" field

  Scenario: Reject request missing required "id" field
    Given an invalid request without "id"
    Then deserialization fails with missing required field error

  Scenario: Reject request missing required "X-Tenant-ID" header
    Given an invalid request without "X-Tenant-ID" header
    Then deserialization fails with missing required field error
