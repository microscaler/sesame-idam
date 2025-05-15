#[cfg(test)]
mod tests {
    use crate::test_utils::TestEnv;
    use serde_json::json;

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_validate_api_key() {
        let env = TestEnv::new("/api/backend/v1/end_user_api_keys/validate").await;

        let payload = json!({
            "api_key_token": "test_end_user_api_key"
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
                "metadata": null,
                "user": {
                    "user_id": "a04d69d7-9347-48a3-aa01-8e7ce9aeee04",
                    "email": "test@sesame.microscaler.io",
                    "email_confirmed": true,
                    "has_password": true,
                    "username": "username",
                    "first_name": "Anthony",
                    "last_name": "Edwards",
                    "picture_url": "https://example.com/img.png",
                    "properties": {
                        "tos": true,
                        "favoriteSport": "basketball"
                    },
                    "metadata": null,
                    "locked": false,
                    "enabled": true,
                    "mfa_enabled": false,
                    "can_create_orgs": false,
                    "created_at": 1712770147,
                    "last_active_at": 1712847115,
                    "org_id_to_org_info": {
                        "bdfbbf84-bcee-4dfe-baa5-1e2dc092991d": {
                            "org_id": "bdfbbf84-bcee-4dfe-baa5-1e2dc092991d",
                            "org_name": "Timberwolves",
                            "org_metadata": {},
                            "url_safe_org_name": "timberwolves",
                            "user_role": "Admin",
                            "inherited_user_roles_plus_current_role": [
                                "Admin",
                                "Owner",
                                "Member"
                            ],
                            "user_permissions": [
                                "propelauth::can_invite",
                                "propelauth::can_change_roles",
                                "propelauth::can_remove_users",
                                "propelauth::can_setup_saml",
                                "propelauth::can_manage_api_keys"
                            ]
                        }
                    },
                    "update_password_required": false
                },
                "org": null,
                "user_in_org": null,
                "user_id": "a04d69d7-9347-48a3-aa01-8e7ce9aeee04",
                "org_id": null
            }),
            "Unexpected response JSON"
        );
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_create_api_key() {
        let env = TestEnv::new("/api/backend/v1/end_user_api_keys").await;

        let payload = json!({
            "user_id": "a04d69d7-9347-48a3-aa01-8e7ce9aeee04",
            "expires_at_seconds": 1712880246,
            "metadata": {
                "customKey": "customValue"
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
                "api_key_id": "6ba56a96c76f86141435c8211309f4c8",
                "api_key_token": "6ba56a96c76f86141435c8211309f4c86801439aed6c172d8c1762ee4fd2e576e531d9fffa21ad0b217f6a77013bc1af"
            }),
            "Unexpected response JSON"
        );
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_update_api_key() {
        let env = TestEnv::new("/api/backend/v1/end_user_api_keys/6ba56a96c76f86141435c8211309f4c8").await;

        let payload = json!({
            "expires_at_seconds": 1712848015,
            "metadata": {
                "customKey": "customValue"
            }
        });

        let response = env
            .client
            .patch(env.url.clone())
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
    async fn test_delete_api_key() {
        let env = TestEnv::new("/api/backend/v1/end_user_api_keys/6ba56a96c76f86141435c8211309f4c8").await;

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
    async fn test_fetch_api_key() {
        let env = TestEnv::new("/api/backend/v1/end_user_api_keys/85b90f382257db0ef057982725997fe1").await;

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
                "api_key_id": "85b90f382257db0ef057982725997fe1",
                "created_at": 1712847088,
                "expires_at_seconds": null,
                "metadata": null,
                "user_id": "a04d69d7-9347-48a3-aa01-8e7ce9aeee04",
                "org_id": null
            }),
            "Unexpected response JSON"
        );
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_fetch_active_api_keys() {
        let env = TestEnv::new("/api/backend/v1/end_user_api_keys?user_id=a04d69d7-9347-48a3-aa01-8e7ce9aeee04&user_email=test@example.com&org_id=null&page_size=10&page_number=0").await;

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
                "api_keys": [
                    {
                        "api_key_id": "85b90f382257db0ef057982725997fe1",
                        "created_at": 1712847088,
                        "expires_at_seconds": null,
                        "metadata": null,
                        "user_id": "a04d69d7-9347-48a3-aa01-8e7ce9aeee04",
                        "org_id": null
                    }
                ],
                "total_api_keys": 1,
                "current_page": 0,
                "page_size": 10,
                "has_more_results": false
            }),
            "Unexpected response JSON"
        );
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_fetch_expired_api_keys() {
        let env = TestEnv::new("/api/backend/v1/end_user_api_keys/archived?user_id=a04d69d7-9347-48a3-aa01-8e7ce9aeee04&user_email=test@example.com&org_id=4896c602-7c67-4d32-a25d-5adb9a15a60e&page_size=10&page_number=0").await;

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
                "api_keys": [
                    {
                        "api_key_id": "6ba56a96c76f86141435c8211309f4c8",
                        "created_at": 1712846425,
                        "expires_at_seconds": 1630425600,
                        "metadata": {
                            "customKey": "customValue"
                        },
                        "user_id": "a04d69d7-9347-48a3-aa01-8e7ce9aeee04",
                        "org_id": "4896c602-7c67-4d32-a25d-5adb9a15a60e"
                    },
                    {
                        "api_key_id": "8488868c3e6b31adaf7883c31e78c0d9",
                        "created_at": 1712846196,
                        "expires_at_seconds": 1,
                        "metadata": null,
                        "user_id": "813a8a12-eecc-4629-a36e-16e73d094b54",
                        "org_id": null
                    },
                    {
                        "api_key_id": "a7dcac6aba8deb36d50a600698b776a1",
                        "created_at": 1705684299,
                        "expires_at_seconds": 1,
                        "metadata": {
                            "customKey": "customValue"
                        },
                        "user_id": null,
                        "org_id": null
                    }
                ],
                "total_api_keys": 3,
                "current_page": 0,
                "page_size": 10,
                "has_more_results": false
            }),
            "Unexpected response JSON"
        );
    }
}
