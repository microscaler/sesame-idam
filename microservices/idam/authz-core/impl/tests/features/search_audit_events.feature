Feature: Search Audit Events
  As an admin or authorized user
  I want to search audit events with filters and sorting
  So that I can find specific audit entries

  Scenario: Search returns empty when no data
    Given a valid tenant
    And no audit events exist
    When I request POST /authz/audit/events/search
    Then the response contains 0 items
    And the response has total_count 0
    And the response has an "items" field of type array
    And the response has a "total" field of type integer
    And the response has a "limit" field of type integer
    And the response has an "offset" field of type integer

  Scenario: Search with event_type filter
    Given a valid tenant
    And audit events exist
    When I request POST /authz/audit/events/search with event_type=authentication
    Then the response returns matching events

  Scenario: Search with actor filter
    Given a valid tenant
    And audit events exist
    When I request POST /authz/audit/events/search with actor=admin
    Then the response returns matching events

  Scenario: Search with combined filters
    Given a valid tenant
    And audit events exist
    When I request POST /authz/audit/events/search with event_type, actor, and severity filters
    Then the response returns matching events

  Scenario: Search with sort_by parameter
    Given a valid tenant
    And audit events exist
    When I request POST /authz/audit/events/search with sort_by=timestamp
    Then the response returns sorted events

  Scenario: Response schema validation
    Given a valid tenant
    When I request POST /authz/audit/events/search
    Then the response body has "items" field
    And the response body has "total" field
    And the response body has "limit" field
    And the response body has "offset" field

  Scenario: Reject request missing required fields
    Given an invalid request without tenant_id and X-Tenant-ID
    Then deserialization fails with missing required field error

  Scenario: Reject request missing "X-Tenant-ID" header
    Given an invalid request without "X-Tenant-ID" header
    Then deserialization fails with missing required field error
