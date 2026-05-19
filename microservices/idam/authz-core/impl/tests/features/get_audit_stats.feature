# BDD Feature: Get Audit Stats
# Endpoint: POST /authz/audit/events/stats

Feature: Get Audit Stats
  As an admin or authorized user
  I want to retrieve aggregated audit statistics
  So that I can monitor audit activity trends

  @audit-stats
  Scenario: Get audit statistics for a tenant
    Given a valid tenant
    When I request POST /authz/audit/events/stats
    Then the response contains a total field
    And the response has field "total"
    And the response has field "by_type"
    And the response has field "by_severity"

  @audit-stats
  Scenario: Reject request missing required fields
    Given an invalid request without tenant_id
    Then deserialization fails with missing required field error
