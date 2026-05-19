Feature: Role Management (Assign/Revoke Principal Role)

  As an admin or service
  I want to assign and revoke roles for principals
  So that authorization decisions can be made based on role membership

  Scenario: Assign a role to a principal
    Given the authz-core service is running
    When I assign a role to a principal
    Then the role assignment response has error field

  Scenario: Revoke a role from a principal
    Given the authz-core service is running
    When I revoke a role from a principal
    Then the role revocation response has error field
