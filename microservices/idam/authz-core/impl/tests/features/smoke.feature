Feature: authz-core smoke test
  As a developer
  I want to verify the service handler is operational

  Scenario: Service handler returns valid response
    Given the authz-core service is running
    When I call the authorize endpoint with a valid request
    Then the response body has field "allowed" set to true

  Scenario: Response structure is valid
    Given the authz-core service is running
    When I call the authorize endpoint with a valid request
    Then the response body has field "allowed"
