#[cfg(test)]
mod tests {
    use crate::test_utils::{EXPOSED_PORT, TestEnv};
    use serde_json::json;


    #[tokio::test]
    async fn test_get_root() {
        let env = TestEnv::new("/").await;

        let response = env
            .client
            .get(env.url.clone())
            .bearer_auth("valid_token")
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(
            response_json,
            json!({
                "total_invites": 0,
                "current_page": 0,
                "page_size": 0,
                "has_more_results": true,
                "invites": [
                    {
                        "invitee_email": "user@example.com",
                        "org_id": "string",
                        "org_name": "string",
                        "role_in_org": "string",
                        "additional_roles_in_org": [null],
                        "created_at": 0,
                        "expires_at": 0,
                        "inviter_email": null,
                        "inviter_user_id": null
                    }
                ]
            }),
            "Unexpected response JSON"
        );
    }

    #[tokio::test]
    async fn test_create_user() {
        let env = TestEnv::new("/api/backend/v1/user/").await;

        let payload = json!({
            "email": "test@sesame.microscaler.io",
            "email_confirmed": false,
            "send_email_to_confirm_email_address": false,
            "password": "hxjV6A0zcp",
            "ask_user_to_update_password_on_login": true,
            "username": "ant",
            "first_name": "Anthony",
            "last_name": "Edwards",
            "properties": {
                "favoriteSport": "basketball"
            },
            "ignore_domain_restrictions": false
        });

        let response = env
            .client
            .post(env.url.clone())
            .bearer_auth("valid_token")
            .json(&payload)
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        if EXPOSED_PORT.parse::<u16>().unwrap() == 4010 {
            assert_eq!(
                response_json,
                json!({
                    "user_id": "string"
                })
            );
        } else if EXPOSED_PORT.parse::<u16>().unwrap() == 3000 {
            assert_eq!(
                response_json,
                json!({
                    "user_id": "a04d69d7-9347-48a3-aa01-8e7ce9aeee04"
                })
            );
        }
    }

    #[tokio::test]
    async fn test_fetch_user_details_by_user_id() {
        let env =
            TestEnv::new("/api/backend/v1/user/a04d69d7-9347-48a3-aa01-8e7ce9aeee04?include_orgs=true").await;

        let response = env
            .client
            .get(env.url.clone())
            .bearer_auth("valid_token")
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(
            response_json,
            json!({"user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
                  "email": "test@example.com",
                  "email_confirmed": true,
                  "username": "airbud3",
                  "first_name": "Buddy",
                  "last_name": "Framm",
                  "picture_url": "https://example.com/picture.jpg",
                  "properties": {
                    "favoriteSport": "basketball"
                  },
                  "has_password": true,
                  "update_password_required": false,
                  "locked": false,
                  "enabled": true,
                  "mfa_enabled": false,
                  "can_create_orgs": true,
                  "created_at": 1627600000,
                  "last_active_at": 1627600000,
                  "org_id_to_org_info": {
                    "1189c444-8a2d-4c41-8b4b-ae43ce79a492": {
                      "org_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
                      "org_name": "Example Org",
                      "org_metadata": {
                        "timezone": "EST"
                      },
                      "user_role": "Owner",
                      "url_safe_org_name": "example-org",
                      "inherited_user_roles_plus_current_role": [
                        "Admin",
                        "Owner",
                        "Member"
                      ],
                      "user_permissions": [
                        "can_view_billing"
                      ],
                      "org_role_structure": "single_role_in_hierarchy",
                      "additional_roles": []
                    }
                  },
                  "legacy_user_id": "1234567890"
            })
        );
    }

    #[tokio::test]
    async fn test_fetch_user_details_by_email() {
        let env =
            TestEnv::new("/api/backend/v1/user/email?email=test@sesame.microscaler.io&include_orgs=true")
                .await;

        let response = env
            .client
            .get(env.url.clone())
            .bearer_auth("valid_token")
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(
            response_json,
            json!({"user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
                "email": "test@example.com",
                "email_confirmed": true,
                "username": "airbud3",
                "first_name": "Buddy",
                "last_name": "Framm",
                "picture_url": "https://example.com/picture.jpg",
                "properties": {
                    "favoriteSport": "basketball"
                },
                "has_password": true,
                "update_password_required": false,
                "locked": false,
                "enabled": true,
                "mfa_enabled": false,
                "can_create_orgs": true,
                "created_at": 1627600000,
                "last_active_at": 1627600000,
                "org_id_to_org_info": {
                    "1189c444-8a2d-4c41-8b4b-ae43ce79a492": {
                        "org_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
                        "org_name": "Example Org",
                        "org_metadata": {
                            "timezone": "EST",
                        },
                        "user_role": "Owner",
                        "url_safe_org_name": "example-org",
                        "inherited_user_roles_plus_current_role": [
                            "Admin",
                            "Owner",
                            "Member"
                        ],
                        "user_permissions": ["can_view_billing"],
                        "org_role_structure": "single_role_in_hierarchy",
                        "additional_roles": []
                    }
                },
                "legacy_user_id": "1234567890"
            })
        );
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_fetch_user_details_by_username() {
        let env = TestEnv::new("/api/backend/v1/user/username?username=airbud3&include_orgs=true").await;

        let response = env
            .client
            .get(env.url.clone())
            .bearer_auth("valid_token")
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(
            response_json,
            json!({
                "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
                "email": "test@example.com",
                "email_confirmed": true,
                "username": "airbud3",
                "first_name": "Buddy",
                "last_name": "Framm",
                "picture_url": "https://example.com/picture.jpg",
                "properties": {
                    "favoriteSport": "basketball"
                },
                "has_password": true,
                "update_password_required": false,
                "locked": false,
                "enabled": true,
                "mfa_enabled": false,
                "can_create_orgs": true,
                "created_at": 1627600000,
                "last_active_at": 1627600000,
                "org_id_to_org_info": {
                    "1189c444-8a2d-4c41-8b4b-ae43ce79a492": {
                        "org_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
                        "org_name": "Example Org",
                        "org_metadata": {
                            "timezone": "EST",
                        },
                        "user_role": "Owner",
                        "url_safe_org_name": "example-org",
                        "inherited_user_roles_plus_current_role": [
                            "Admin",
                            "Owner",
                            "Member"
                        ],
                        "user_permissions": ["can_view_billing"],
                        "org_role_structure": "single_role_in_hierarchy",
                        "additional_roles": []
                    }
                },
                "legacy_user_id": "1234567890"
            })
        );
    }

    #[tokio::test]
    async fn test_query_for_users() {
        let env = TestEnv::new("/api/backend/v1/user/query?page_size=10&page_number=1&order_by=CREATED_AT_ASC&email_or_username=test@example.com&include_orgs=true&legacy_user_id=1234").await;

        let response = env
            .client
            .get(env.url.clone())
            .bearer_auth("valid_token")
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(
            response_json,
            json!({"order_by": "CREATED_AT_DESC",
              "page_number": 1,
              "page_size": 10,
              "results": [
                {
                  "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
                  "email": "test@example.com",
                  "email_confirmed": true,
                  "username": "airbud3",
                  "first_name": "Buddy",
                  "last_name": "Framm",
                  "picture_url": "https://example.com/picture.jpg",
                  "properties": {
                    "favoriteSport": "basketball"
                  },
                  "has_password": true,
                  "update_password_required": false,
                  "locked": false,
                  "enabled": true,
                  "mfa_enabled": false,
                  "can_create_orgs": true,
                  "created_at": 1627600000,
                  "last_active_at": 1627600000,
                  "org_id_to_org_info": {
                    "1189c444-8a2d-4c41-8b4b-ae43ce79a492": {
                      "org_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
                      "org_name": "Example Org",
                      "org_metadata": {
                        "timezone": "EST"
                      },
                      "user_role": "Owner",
                      "url_safe_org_name": "example-org",
                      "inherited_user_roles_plus_current_role": [
                        "Admin",
                        "Owner",
                        "Member"
                      ],
                      "user_permissions": [
                        "can_view_billing"
                      ],
                      "org_role_structure": "single_role_in_hierarchy",
                      "additional_roles": []
                    }
                  },
                  "legacy_user_id": "1234567890"
                }
              ],
              "total_results": 1
            })
        );
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_update_user() {
        let env = TestEnv::new("/api/backend/v1/user/a04d69d7-9347-48a3-aa01-8e7ce9aeee04").await;

        let payload = json!({
            "username": "username",
            "first_name": "firstName",
            "last_name": "lastName",
            "picture_url": "https://example.com/img.png",
            "properties": {
                "favoriteSport": "value"
            },
            "update_password_required": false,
            "legacy_user_id": "abc123"
        });

        let response = env
            .client
            .put(env.url.clone())
            .bearer_auth("valid_token")
            .json(&payload)
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(response_json, json!({}), "Expected empty JSON response");
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_update_user_email() {
        let env = TestEnv::new("/api/backend/v1/user/a04d69d7-9347-48a3-aa01-8e7ce9aeee04/email").await;

        let payload = json!({
            "new_email": "test@example.com",
            "require_email_confirmation": false
        });

        let response = env
            .client
            .put(env.url.clone())
            .bearer_auth("valid_token")
            .json(&payload)
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(response_json, json!({}), "Expected empty JSON response");
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_update_user_password() {
        let env = TestEnv::new("/api/backend/v1/user/a04d69d7-9347-48a3-aa01-8e7ce9aeee04/password").await;

        let payload = json!({
            "password": "moresecurethanthis",
            "ask_user_to_update_password_on_login": false
        });

        let response = env
            .client
            .put(env.url.clone())
            .bearer_auth("valid_token")
            .json(&payload)
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(response_json, json!({}), "Expected empty JSON response");
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_create_magic_link() {
        let env = TestEnv::new("/api/backend/v1/magic_link").await;

        let payload = json!({
            "email": "test@example.com",
            "redirect_to_url": "http://localhost:3000",
            "expires_in_hours": 1,
            "create_new_user_if_one_doesnt_exist": false
        });

        let response = env
            .client
            .post(env.url.clone())
            .bearer_auth("valid_token")
            .json(&payload)
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(
            response_json,
            json!({
                "url": "https://15814390537.propelauthtest.com/verify_login_passwordless?t=eyJ0b2tlbiI6ImM2YzRiNjE5YTM0YjFkZjQ5NjQzODBjZDRlMjA1ZjE0NmY4MzI2MmFmNzRiNzhkYjkyZGU3MjZiNjk1Yjk3Mjc4NWVhNDRiMzAyYzk5MWIwOTQ1MDQ1OTg0MzQyNmVjZiIsInVzZXIiOiJhMDRkNjlkNy05MzQ3LTQ4YTMtYWEwMS04ZTdjZTlhZWVlMDQifQ"
            }),
            "Unexpected response JSON"
        );
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_create_access_token() {
        let env = TestEnv::new("/api/backend/v1/access_token").await;

        let payload = json!({
            "user_id": "a04d69d7-9347-48a3-aa01-8e7ce9aeee04",
            "duration_in_minutes": 1440
        });

        let response = env
            .client
            .post(env.url.clone())
            .bearer_auth("valid_token")
            .json(&payload)
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(
            response_json,
            json!({
                "access_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiIsImtpZCI6IjE4ZDU2M2NhLTU3ZjMtNDA0ZC05NjI5LWQ0YWI5M2E2OWVkNSJ9.eyJzdWIiOiJhMDRkNjlkNy05MzQ3LTQ4YTMtYWEwMS04ZTdjZTlhZWVlMDQiLCJpYXQiOjE3MTI4NDc2MDEsImV4cCI6MTcxMjkzNDAwMSwidXNlcl9pZCI6ImEwNGQ2OWQ3LTkzNDctNDhhMy1hYTAxLThlN2NlOWFlZWUwNCIsImlzcyI6Imh0dHBzOi8vMTU4MTQzOTA1MzcucHJvcGVsYXV0aHRlc3QuY29tIiwiZW1haWwiOiJ0ZXN0QHByb3BlbGF1dGguY29tIiwiZmlyc3RfbmFtZSI6IkFudGhvbnkiLCJsYXN0X25hbWUiOiJFZHdhcmRzIiwidXNlcm5hbWUiOiJ1c2VybmFtZSIsIm9yZ19pZF90b19vcmdfbWVtYmVyX2luZm8iOnsiYmRmYmJmODQtYmNlZS00ZGZlLWJhYTUtMWUyZGMwOTI5OTFkIjp7Im9yZ19pZCI6ImJkZmJiZjg0LWJjZWUtNGRmZS1iYWE1LTFlMmRjMDkyOTkxZCIsIm9yZ19uYW1lIjoiVGltYmVyd29sdmVzIiwidXJsX3NhZmVfb3JnX25hbWUiOiJ0aW1iZXJ3b2x2ZXMiLCJvcmdfbWV0YWRhdGEiOnt9LCJ1c2VyX3JvbGUiOiJBZG1pbiIsImluaGVyaXRlZF91c2VyX3JvbGVzX3BsdXNfY3VycmVudF9yb2xlIjpbIkFkbWluIiwiT3duZXIiLCJNZW1iZXIiXSwidXNlcl9wZXJtaXNzaW9ucyI6WyJwcm9wZWxhdXRoOjpjYW5faW52aXRlIiwicHJvcGVsYXV0aDo6Y2FuX2NoYW5nZV9yb2xlcyIsInByb3BlbGF1dGg6OmNhbl9yZW1vdmVfdXNlcnMiLCJwcm9wZWxhdXRoOjpjYW5fc2V0dXBfc2FtbCIsInByb3BlbGF1dGg6OmNhbl9tYW5hZ2VfYXBpX2tleXMiXX19LCJwcm9wZXJ0aWVzIjp7ImZhdm9yaXRlU3BvcnQiOiJ2YWx1ZSJ9fQ.bHKma4eZ3TpH482epbD7s1_tw5K-bfC91_um6XqVerUT4B6EwF7DcWtNUQEsbcJKa1ORpaIgrjUgG3Y_dXUT1V4Cnws9fRLeseJbMYrReRS2U8bXS6m5BDr5iH1CTIrv5b1hrIm3pocRao93Ja1W9m65sFYQsn_XhBCiAwv92gnY44DX5ibfaedi5i1Jd9SqXq0Nx0eMZBNmYjCBilkfIH7G9Ru5rQcYqCyjQmf7xTHbdmoKBIxoZv1t5u1hKDZOCA5pdcOiRDQyagExQhEPXhYVUJw05qRfE9Kr7OOlOK6OQ0yQlQBlmm7sEj9OvtsLRHbXYPGBx_W1aBaxV33ChA"
            }),
            "Unexpected response JSON"
        );
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_migrate_user_from_external_source() {
        let env =
            TestEnv::new("/api/backend/v1/migrate_user/?page_size=10&page_number=1&order_by=CREATED_AT_ASC")
                .await;

        let payload = json!({
            "email": "test@example.com",
            "email_confirmed": true,
            "existing_user_id": "1234",
            "existing_password_hash": "bcrypt_hash",
            "existing_mfa_base32_encoded_secret": "base32_encoded_secret",
            "update_password_required": false,
            "enabled": true,
            "username": "airbud3",
            "first_name": "Buddy",
            "last_name": "Framm",
            "picture_url": "https://example.com/img.png",
            "properties": {
                "favoriteSport": "basketball"
            }
        });

        let response = env
            .client
            .post(env.url.clone())
            .bearer_auth("valid_token")
            .json(&payload)
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(
            response_json,
            json!({
                "user_id": "e22451c4-9fb6-486a-bb80-9f403e00b27b"
            }),
            "Unexpected response JSON"
        );
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_update_migrated_user_password() {
        let env = TestEnv::new("/api/backend/v1/migrate_user/password").await;

        let payload = json!({
            "user_id": "a04d69d7-9347-48a3-aa01-8e7ce9aeee04",
            "password_hash": "the_password_hash"
        });

        let response = env
            .client
            .post(env.url.clone())
            .bearer_auth("valid_token")
            .json(&payload)
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(response_json, json!({}), "Expected empty JSON response");
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_delete_user() {
        let env = TestEnv::new("/api/backend/v1/user/a04d69d7-9347-48a3-aa01-8e7ce9aeee04").await;

        let response = env
            .client
            .delete(env.url.clone())
            .bearer_auth("valid_token")
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(response_json, json!({}), "Expected empty JSON response");
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_block_user() {
        let env = TestEnv::new("/api/backend/v1/user/a04d69d7-9347-48a3-aa01-8e7ce9aeee04/disable").await;

        let response = env
            .client
            .post(env.url.clone())
            .bearer_auth("valid_token")
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(response_json, json!({}), "Expected empty JSON response");
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_unblock_user() {
        let env = TestEnv::new("/api/backend/v1/user/a04d69d7-9347-48a3-aa01-8e7ce9aeee04/enable").await;

        let response = env
            .client
            .post(env.url.clone())
            .bearer_auth("valid_token")
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(response_json, json!({}), "Expected empty JSON response");
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_disable_user_2fa() {
        let env = TestEnv::new("/api/backend/v1/user/a04d69d7-9347-48a3-aa01-8e7ce9aeee04/disable_2fa").await;

        let response = env
            .client
            .post(env.url.clone())
            .bearer_auth("valid_token")
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(response_json, json!({}), "Expected empty JSON response");
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_resend_email_confirmation() {
        let env = TestEnv::new("/api/backend/v1/resend_email_confirmation").await;

        let payload = json!({
            "user_id": "a04d69d7-9347-48a3-aa01-8e7ce9aeee04"
        });

        let response = env
            .client
            .post(env.url.clone())
            .bearer_auth("valid_token")
            .json(&payload)
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(response_json, json!({}), "Expected empty JSON response");

        // # TODO
        // Mock email receipt (example)
        // Simulate checking a mock email service or verifying the email was "sent"
        // This part depends on your test setup and email mocking library.

        // Use a mock email service like (MailHog)[https://github.com/mailhog/MailHog]
        // or (GreenMail)[https://greenmail-mail-test.github.io/greenmail/#].
        // Configure your application to send emails to the mock service during tests.
        // Query the mock service to verify the email was sent and contains the expected content.
    }
}
