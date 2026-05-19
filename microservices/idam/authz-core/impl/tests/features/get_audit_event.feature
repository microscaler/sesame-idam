# BDD Feature: Get Audit Event
# Endpoint: GET /authz/audit/events/{id}

Feature: Get Audit Event
  As an admin or authorized user
  I want to retrieve a single audit event by ID
  So that I can review audit trail details

  @audit-event
  Scenario: Retrieve audit event by ID
    Given a valid tenant
    And an audit event with id
    When I request GET /authz/audit/events
    Then the response contains an id field
    And the response has field "actor"
    And the response has field "event_action"
    And the response has field "event_type"
    And the response has field "ip_address"
    And the response has field "timestamp"

  @audit-event
  Scenario: Reject request missing required fields
    Given an invalid request without id
    And an invalid request without x-tenant-id header
    Then deserialization fails with missing required field error
