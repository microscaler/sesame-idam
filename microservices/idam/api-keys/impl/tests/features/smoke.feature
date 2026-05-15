Feature: api-keys healthcheck
  As a developer
  I want to verify the service is running

  @smoke
  Scenario: Service healthcheck
    Given the service is running
    When I check the health endpoint
    Then I should receive a 200 OK
