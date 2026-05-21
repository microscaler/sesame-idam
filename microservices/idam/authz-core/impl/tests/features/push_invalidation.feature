Feature: Push Invalidation Events
  As a Sesame-IDAM service consumer
  I want to receive version bump events via Redis pub/sub
  So that I can invalidate cached versions without waiting for polling

  Scenario: Publisher is created when Redis is configured
    Given the authz-core service has Redis configured
    When I create the push invalidation publisher
    Then the publisher is created successfully
    And the publisher has the correct HMAC secret

  Scenario: Publisher wrapper can publish tenant-wide event
    Given the publisher is initialized with Redis URL and HMAC secret
    When I call publish_tenant with tenant_id "hauliage", new_version 42, reason "role_revoked"
    Then the publisher spawns an async task
    And errors are logged if Redis connection fails (fire-and-forget)

  Scenario: Publisher wrapper can publish subject-specific event
    Given the publisher is initialized with Redis URL and HMAC secret
    When I call publish_subject with tenant_id "hauliage", user_id "alice", new_version 43, reason "user_disabled"
    Then the publisher spawns an async task
    And the event includes the user_id field

  Scenario: Subscriber metrics are registered
    Given I create a SubscriberConfig with a Prometheus registry
    When I create a VersionBumpSubscriber from the config
    Then the subscriber has version_bump_total counter registered
    And the subscriber has revocation_propagation_seconds histogram registered

  Scenario: Subscriber handles tenant-wide event correctly
    Given a subscriber is created and cache is empty
    When a valid tenant-wide version bump event is received
    Then the tenant cache entry is updated
    And no user-specific cache entry is created

  Scenario: Subscriber handles subject-specific event correctly
    Given a subscriber is created and cache is empty
    When a valid subject-specific version bump event is received
    Then the tenant cache entry is updated
    And the user-specific cache entry is also updated
