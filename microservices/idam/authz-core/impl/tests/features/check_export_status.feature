# BDD Feature: Check Export Status
# Endpoint: GET /authz/audit/events/export/{export_id}

Feature: Check Export Status
  As an admin or authorized user
  I want to poll the status of an export job
  So that I know when to download the exported file

  @export-status
  Scenario Outline: Check status of a pending export
    Given a valid tenant with ID "<tenant_id>"
    And an export job with ID "<export_id>"
    When I request GET /authz/audit/events/export/{export_id} with X-Tenant-ID header
    Then the response status code is 200
    And the response body contains:
      | field                | type   |
      | export_id            | string |
      | status               | string |
      | estimated_completion | string? |
      | download_url         | string? |

    Examples:
      | tenant_id                              | export_id                              |
      | 6ba7b810-9dad-11d1-80b4-00c04fd430c8 | 7f3c2a1b-8e9d-4f5a-b6c7-d8e9f0a1b2c3 |

  @export-status
  Scenario: Check status returns pending
    Given an export job with ID "7f3c2a1b-8e9d-4f5a-b6c7-d8e9f0a1b2c3"
    When I request GET /authz/audit/events/export/{export_id}
    Then the response "status" is "pending"

  @export-status
  Scenario: Reject request missing required "export_id" field
    Given an invalid request without "export_id"
    When I attempt to deserialize the request
    Then deserialization fails with missing required field error

  @export-status
  Scenario: Reject request missing required "X-Tenant-ID" header
    Given an invalid request without "X-Tenant-ID" header
    When I attempt to deserialize the request
    Then deserialization fails with missing required field error

  @export-status
  Scenario: Verify response "export_id" is string
    Given a valid request
    When I request GET /authz/audit/events/export/{export_id}
    Then the response "export_id" is a string

  @export-status
  Scenario: Verify response "status" is string
    Given a valid request
    When I request GET /authz/audit/events/export/{export_id}
    Then the response "status" is a string

  @export-status
  Scenario: Verify tenant isolation
    Given a request with X-Tenant-ID header "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    When I construct the request
    Then x_tenant_id is extracted from the header
