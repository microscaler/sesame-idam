Feature: Asymmetric JWKS endpoint (Story 1.1)
  As a JWT consumer
  I want to fetch Ed25519 public keys in standard JWKS format
  So that I can validate tokens signed by identity-login-service

  Scenario: JWKS endpoint returns keys in standard format
    Given the service is running
    When I request /.well-known/jwks.json
    Then I should receive a 200 OK
    And the response body has a "keys" array
    And the "keys" array contains at least one key object

  Scenario: JWKS key has required fields
    Given the service is running
    When I request /.well-known/jwks.json
    Then the first key object has a "kty" field
    And the "kty" field value is "OKP"
    And the first key object has a "kid" field
    And the "kid" field matches pattern "key-[0-9]{4}-[0-9]{2}-[0-9]{2}"
    And the first key object has a "alg" field
    And the "alg" field value is "EdDSA"
    And the first key object has a "use" field
    And the "use" field value is "sig"
    And the first key object has a "crv" field
    And the "crv" field value is "Ed25519"
    And the first key object has a "x" field
    And the "x" field is a non-empty base64url-encoded string

  Scenario: JWKS has Cache-Control header
    Given the service is running
    When I request /.well-known/jwks.json
    Then the response includes a "Cache-Control" header
    And the "Cache-Control" header contains "max-age"

  Scenario: Key rotation creates new key
    Given the service is running
    When the KeyManager prepares a new key
    Then the JWKS contains the original key and the new key
    And both keys are served in the "keys" array
