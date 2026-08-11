# Series A pre-auth tenant resolution map — 2026-08-11

## Target pattern
`ClientRegistry::resolve_pre_auth(client_id, optional X-Tenant-ID)` → `ClientBinding`;
header mismatch / unknown client → `ClientRegistryError::Unknown`.

## DONE (OpenAPI + impl ClientRegistry)
| op | path | notes |
|---|---|---|
| auth_login | POST /auth/login | body `client_id` required; X-Tenant optional |
| auth_register | POST /auth/register | body `client_id` required; X-Tenant optional |
| signup_validate | GET /auth/signup/validate | query `client_id` required; X-Tenant optional |
| social_login | GET /auth/social/{provider}/login | query `client_id`; tenant+client_id in OAuth state |
| auth_forgot_password | POST /auth/password/forgot | body `client_id` required |
| auth_reset_password | POST /auth/password/reset | body `client_id` required |

## PARTIAL
| op | status |
|---|---|
| social_callback | Tenant from OAuth state (not ClientRegistry); X-Tenant optional match |
| auth_token | `client_id` in body (grant-dependent); still header-bound for auth_code + client_credentials (ApiKeyService, not ClientRegistry) |
| auth_session_code | Header tenant + token verify; could derive from access token |
| auth_logout | Header for audit only; Bearer present; desc still says Requires X-Tenant |

## TODO (header/hint only; no client_id in schema; no ClientRegistry)
login_email_otp, login_phone_otp, login_dual_otp (stub), verify_email_otp, verify_phone_otp, verify_dual_otp (stub), magic_link_send, magic_link_verify, sms_magic_link_send, sms_magic_link_verify (stub)

## OpenAPI drift
Many remaining OTP/magic-link ops still describe "Requires X-Tenant-ID" while parameter anchors `id007`/`id033` are `required: false`.
