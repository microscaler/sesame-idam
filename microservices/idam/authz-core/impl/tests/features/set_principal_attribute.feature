Feature: Set Principal Attribute (POST /authz/principals/attributes)

  As a service or admin
  I want to set an ABAC attribute on a principal
  So that I can tag users/orgs with metadata used for authorization decisions

  Scenario: Set a string attribute on a user
    Given the authz-core service is running
    When I send a valid request to set a principal attribute
    Then the attribute response has error field

  Scenario: Attribute setting emits audit event
    Given the authz-core service is running
    When I send a valid request to set a principal attribute
    Then the attribute response has error field
