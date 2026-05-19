# BDD Feature: Export Audit Events
# Endpoint: POST /authz/audit/events/export

Feature: Export Audit Events
  As an admin or authorized user
  I want to export audit events in a specific format
  So that I can perform offline analysis or compliance reporting

  @export
  Scenario: Export audit events
    Given a valid tenant
    When I request POST /authz/audit/events/export
    Then the response contains an export_id field
    And the response has field "export_id"
    And the response has field "status"
    And the response has field "estimated_completion"
    And the response has field "download_url"

  @export
  Scenario: Reject request missing required fields
    Given an invalid request without format
    And an invalid request without x-tenant-id header
    And an invalid request without tenant_id
    Then deserialization fails with missing required field error
