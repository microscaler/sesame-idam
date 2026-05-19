/// Controller handlers for Authentication (login, register, social OAuth, OTP, passwordless).
///
/// Each controller corresponds to a single API endpoint. Controllers audit every
/// request via the global `EMITTER`, then delegate to the service layer.

pub mod auth_forgot_password;
pub mod auth_login;
pub mod login_dual_otp;
pub mod login_email_otp;
pub mod login_phone_otp;
pub mod auth_logout;
pub mod auth_register;
pub mod auth_reset_password;
pub mod auth_token;
pub mod verify_dual_otp;
pub mod verify_email_otp;
pub mod verify_phone_otp;
pub mod oauth_authorize;
pub mod social_callback;
pub mod social_login;
pub mod signup_validate;
pub mod magic_link_send;
pub mod magic_link_verify;
pub mod sms_magic_link_send;
pub mod sms_magic_link_verify;
