Feature: Version Mismatch Handling (Story 5.5)

  As an authz-core service
  I want to detect and deny stale authorization tokens
  So that users with outdated permission snapshots are forced to re-authenticate

  Background:
    Given the version cache contains authz_ver:{user} = N
    And the JWT contains claims.ver = M

  Scenario: Stale token denied with 401 and retry_after
    Given claims.ver = 42 and cached_ver = 45
    When a high-risk request arrives with the stale JWT
    THEN the response status is 401 Unauthorized
    AND the JSON body contains "error": "stale_auth_token"
    AND the JSON body contains "reason": "stale_authz_snapshot"
    AND the JSON body contains "retry_after": 300
    AND the Retry-After HTTP header is "300"
    AND the WWW-Authenticate header contains Bearer error="stale_auth_token", retry_after=300

  Scenario: Client refreshes and retries successfully
    GIVEN user bob receives 401 stale_auth_token with retry_after = 300
    WHEN the client calls POST /auth/refresh with the refresh token
    THEN the response contains a new access token with updated version
    AND the client retries the original request with the new token
    AND the version check passes (new token ver >= cached_ver)
    AND the request succeeds with 200 OK

  Scenario: Client with expired refresh redirected to login
    GIVEN user carol receives 401 stale_auth_token
    WHEN the client tries to refresh but the refresh token is expired
    THEN the refresh fails
    AND the client is redirected to login (re-authentication required)

  Scenario: Large gap requires immediate re-auth
    GIVEN user dave has authz_ver = 200 (version jumped by 150)
    WHEN a request arrives with claims.ver = 50 (gap = 150)
    THEN the response status is 401 Unauthorized
    AND the response body contains "retry_after": 0
    AND the Retry-After HTTP header is "0"

  Scenario: jwt-only route skips version checking
    GIVEN user eve has authz_ver = 100
    WHEN a jwt-only route request arrives with claims.ver = 1
    THEN no version check is performed
    AND the request proceeds normally

  Scenario: Token with ver >= cached_ver succeeds
    GIVEN user frank has authz_ver = 10
    WHEN a high-risk request arrives with claims.ver = 10
    THEN the version check passes (10 >= 10)
    AND the request proceeds normally

  Scenario: Version mismatch metrics recorded
    GIVEN a high-risk request with stale token (ver < cached_ver)
    THEN the metric version_mismatch_total{result="small"} is incremented
    AND the metric version_lookup_latency_ms records the cache lookup latency

  Scenario: Retry-After header matches JSON body
    GIVEN a version mismatch with gap = 7
    WHEN the response is generated
    THEN the Retry-After HTTP header value is "300"
    AND the retry_after JSON body field is 300
    AND both values are consistent

  Scenario: WWW-Authenticate header present in error response
    GIVEN a version mismatch occurs
    WHEN the response is parsed
    THEN the WWW-Authenticate header contains Bearer error="stale_auth_token"
    AND the WWW-Authenticate header contains retry_after=300

  Scenario: Stale token denied on admin route
    GIVEN admin user has authz_ver = 25
    WHEN a request to create org arrives with claims.ver = 20 (gap = 5)
    THEN the admin action is denied with 401 stale_auth_token
    AND retry_after = 300
