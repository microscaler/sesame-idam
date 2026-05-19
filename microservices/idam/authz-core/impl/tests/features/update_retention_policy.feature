# BDD Feature: Update Retention Policy
# Endpoint: PATCH /authz/audit/events/retention

Feature: Update Retention Policy
  As an admin or authorized user
  I want to configure how long audit events are retained
  So that I can comply with data governance requirements

  @retention
  Scenario Outline: Update retention policy with retention_days
    Given a valid tenant with ID "<tenant_id>"
    When I request PATCH /authz/audit/events/retention with retention_days set to <retention_days>
    And I include the X-Tenant-ID header
    Then the response status code is 200
    And the response body contains:
      | field                | type    |
      | tenant_id            | string  |
      | retention_days       | int     |
      | id                   | string? |
      | event_type           | string? |
      | archive_after_days   | int?    |
      | delete_after_days    | int?    |
      | created_at           | string? |

    Examples:
      | tenant_id                              | retention_days |
      | 6ba7b810-9dad-11d1-80b4-00c04fd430c8 | 365            |

  @retention
  Scenario: Update policy with optional "id" field
    Given a valid request with "id" set to "retention-rule-1"
    When I request PATCH /authz/audit/events/retention
    Then the response includes "tenant_id" and "retention_days" fields

  @retention
  Scenario: Update policy with optional "event_type" field
    Given a valid request with "event_type" set to "authentication"
    When I request PATCH /authz/audit/events/retention
    Then the response includes "tenant_id" field

  @retention
  Scenario: Update policy with optional "archive_after_days" field
    Given a valid request with "archive_after_days" set to 90
    When I request PATCH /authz/audit/events/retention
    Then the response includes "tenant_id" field

  @retention
  Scenario: Update policy with optional "delete_after_days" field
    Given a valid request with "delete_after_days" set to 730
    When I request PATCH /authz/audit/events/retention
    Then the response includes "tenant_id" field

  @retention
  Scenario: Update policy with all optional fields
    Given a valid request with all optional fields populated
    When I request PATCH /authz/audit/events/retention
    Then the response includes all optional fields

  @retention
  Scenario: Reject request missing required "retention_days" field
    Given an invalid request without "retention_days"
    When I attempt to deserialize the request
    Then deserialization fails with missing required field error

  @retention
  Scenario: Reject request missing required "X-Tenant-ID" header
    Given an invalid request without "X-Tenant-ID" header
    When I attempt to deserialize the request
    Then deserialization fails with missing required field error

  @retention
  Scenario: Reject request missing required "tenant_id" field
    Given an invalid request without "tenant_id"
    When I attempt to deserialize the request
    Then deserialization fails with missing required field error

  @retention
  Scenario: Verify response "retention_days" is integer
    Given a valid request with retention_days set to 365
    When I request PATCH /authz/audit/events/retention
    Then the response "retention_days" is an integer

  @retention
  Scenario: Verify tenant isolation
    Given a request with X-Tenant-ID header "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    When I construct the request
    Then x_tenant_id and tenant_id are extracted from the header
