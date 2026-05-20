Feature: Create Retention Policy
  As an admin or authorized user
  I want to create audit log retention policies
  So that I can comply with data governance requirements

  Scenario: Create policy with all fields
    Given a valid tenant
    When I request POST /authz/audit/events/retention with event_type=authentication, retention_days=365, archive_after_days=180, delete_after_days=365
    Then the response has id field (generated UUID)
    And the response has event_type field of type string
    And the response has retention_days field of type integer
    And the response has archive_after_days field of type integer
    And the response has delete_after_days field of type integer
    And the response has tenant_id field of type string
    And the response may have created_at field of type string

  Scenario: Create policy with required fields only
    Given a valid tenant
    When I request POST /authz/audit/events/retention with event_type=audit, retention_days=90
    Then the response has a generated id
    And the response has retention_days=90
    And optional fields (archive_after_days, delete_after_days) are null

  Scenario: Response "id" is a non-empty string
    Given a valid tenant
    When I request POST /authz/audit/events/retention with event_type=audit, retention_days=90
    Then the response "id" field is a non-empty string

  Scenario: Response "retention_days" is an integer
    Given a valid tenant
    When I request POST /authz/audit/events/retention
    Then the response "retention_days" is an integer

  Scenario: Reject request missing required "event_type" field
    Given an invalid request without "event_type"
    Then deserialization fails with missing required field error

  Scenario: Reject request missing required "retention_days" field
    Given an invalid request without "retention_days"
    Then deserialization fails with missing required field error

  Scenario: Reject request missing required "X-Tenant-ID" header
    Given an invalid request without "X-Tenant-ID" header
    Then deserialization fails with missing required field error
