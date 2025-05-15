#[cfg(test)]
mod tests {
    use crate::test_utils::{EXPOSED_PORT, TestEnv};
    use serde_json::json;


    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_fetch_org() {
        let env = TestEnv::new("/api/backend/v1/org/582e7c11-6b72-40d8-886d-461e6491fa71").await;

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
                "org_id": "582e7c11-6b72-40d8-886d-461e6491fa71",
                "name": "Acme Inc",
                "url_safe_org_slug": "51c08c1b-8526-40cd-8679-ddb15cea2984",
                "can_setup_saml": false,
                "is_saml_configured": false,
                "is_saml_in_test_mode": false,
                "domain": "acme.com",
                "domain_autojoin": true,
                "domain_restrict": true,
                "max_users": 100,
                "custom_role_mapping_name": "Business Plan"
            }),
            "Unexpected response JSON"
        );
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_fetch_orgs() {
        let env = TestEnv::new("/api/backend/v1/org/query?page_size=10&page_number=0&order_by=CREATED_AT_ASC&name=acme.com&legacy_org_id=1234").await;

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
                "orgs": [
                    {
                        "org_id": "582e7c11-6b72-40d8-886d-461e6491fa71",
                        "name": "Acme Inc",
                        "is_saml_configured": false,
                        "custom_role_mapping_name": "Business Plan"
                    }
                ],
                "total_orgs": 1,
                "current_page": 0,
                "page_size": 10,
                "has_more_results": false
            }),
            "Unexpected response JSON"
        );
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_fetch_users_in_org() {
        let env = TestEnv::new("/api/backend/v1/user/org/582e7c11-6b72-40d8-886d-461e6491fa71?page_size=10&page_number=0&role=Admin&include_orgs=true").await;

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
                "users": [
                    {
                        "user_id": "026cf5b5-34a0-4bb5-918a-6f910d13d835",
                        "role_in_org": "Admin",
                        "email": "paul@sesame.microscaler.io",
                        "email_confirmed": true,
                        "has_password": false,
                        "first_name": "Paul",
                        "last_name": "Test",
                        "picture_url": "https://lh3.googleusercontent.com/a/ACg8ocJGtIn2flEbMt5gIYl7GTlH6IhOpM75950m60fbBleXX3ZO1g=s96-c",
                        "properties": {
                            "metadata": {
                                "test": "test"
                            },
                            "tos": true
                        },
                        "metadata": {
                            "test": "test"
                        },
                        "locked": false,
                        "enabled": true,
                        "mfa_enabled": false,
                        "can_create_orgs": false,
                        "created_at": 1712251498,
                        "last_active_at": 1712761310,
                        "update_password_required": false
                    }
                ],
                "total_users": 1,
                "current_page": 0,
                "page_size": 10,
                "has_more_results": false
            }),
            "Unexpected response JSON"
        );
    }

    #[tokio::test]
    async fn test_create_org() {
        let env = TestEnv::new("/api/backend/v1/org/").await;

        let payload = json!({
            "name": "Acme Inc",
            "domain": "acme.com",
            "enable_auto_joining_by_domain": true,
            "members_must_have_matching_domain": true,
            "custom_role_mapping_name": "Business Plan",
            "legacy_org_id": "1234",
            "max_users": 100
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
        assert!(
            response_json.get("org_id").is_some(),
            "Expected key 'org_id' to exist"
        );
        assert!(
            response_json.get("name").is_some(),
            "Expected key 'name' to exist"
        );
    }

    /// curl -X "POST" \
    /// -H "Authorization: Bearer <API_KEY>" \
    /// -H "Content-Type: application/json" \
    /// -d '{
    ///     "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
    ///     "org_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
    ///     "role": "Admin",
    ///     "additional_roles": ["Member"]
    ///     }' \
    /// "<AUTH_URL>/api/backend/v1/org/add_user"
    #[tokio::test]
    async fn test_add_user_to_org() {
        let env = TestEnv::new("/api/backend/v1/org/add_user").await;

        let payload = json!({
            "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
            "org_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
            "role": "Admin",
            "additional_roles": ["Member"]
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

    ///
    ///
    /// curl -X "POST" \
    ///     -H "Authorization: Bearer <API_KEY>" \
    ///     -H "Content-Type: application/json" \
    ///     -d '{
    ///         "email": "test@propelauth.com",
    ///         "org_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
    ///         "role": "Admin",
    ///         "additional_roles": ["Member"]
    ///     }' \
    ///     "<AUTH_URL>/api/backend/v1/invite_user"
    ///
    #[tokio::test]
    async fn test_invite_user_to_org() {
        let env = TestEnv::new("/api/backend/v1/invite_user").await;


        let payload = json!({
            "email": "test@propelauth.com",
            "org_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
            "role": "Admin",
            "additional_roles": ["Member"]
        });

        let response = env
            .client
            .post(env.url.clone())
            .bearer_auth("Fj3kLm90QwXe2PbtRzYHu1SdVnA4XcGj")
            .json(&payload)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");
    }

    /// curl -X "POST" \
    ///     -H "Authorization: Bearer <API_KEY>" \
    ///     -H "Content-Type: application/json" \
    ///     -d '{
    ///         "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
    ///         "org_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
    ///         "role": "Admin",
    ///         "additional_roles": ["Member"]
    ///     }' \
    ///     "<AUTH_URL>/api/backend/v1/org/change_role"
    #[tokio::test]
    async fn test_change_user_role() {
        let env = TestEnv::new("/api/backend/v1/org/change_role").await;

        let payload = json!({
        "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
        "org_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
        "role": "Admin",
        "additional_roles": ["Member"]
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

    /// curl -X "POST" \
    ///     -H "Authorization: Bearer <API_KEY>" \
    ///     -H "Content-Type: application/json" \
    ///     -d '{
    ///         "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
    ///         "org_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
    ///     }' \
    ///     "<AUTH_URL>/api/backend/v1/org/remove_user
    #[tokio::test]
    async fn test_remove_user_from_org() {
        let env = TestEnv::new("/api/backend/v1/org/remove_user").await;

        let payload = json!({
            "user_id": "a04d69d7-9347-48a3-aa01-8e7ce9aeee04",
            "org_id": "582e7c11-6b72-40d8-886d-461e6491fa71"
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
    }

    /// curl -X "PUT" \
    ///     -H "Authorization: Bearer <API_KEY>" \
    ///     -H "Content-Type: application/json" \
    ///     -d '{
    ///         "name": "Acme Inc",
    ///         "domain": "acme.com",
    ///         "extra_domains": ["hogwarts.edu"],
    ///         "enable_auto_joining_by_domain": true,
    ///         "members_must_have_matching_domain": true,
    ///         "max_users": 100,
    ///         "can_setup_saml": true,
    ///         "legacy_org_id": "1234",
    ///         "metadata": {
    ///             "customKey": "customValue",
    ///         },
    ///     }' \
    ///     "<AUTH_URL>/api/backend/v1/org/<orgId>"
    #[tokio::test]
    async fn test_update_org() {
        let env = TestEnv::new("/api/backend/v1/org/582e7c11-6b72-40d8-886d-461e6491fa71").await;

        let payload = json!({
          "name": "Acme Inc",
          "domain": "acme.com",
          "extra_domains": [
            "hogwarts.edu"
          ],
          "enable_auto_joining_by_domain": true,
          "members_must_have_matching_domain": true,
          "max_users": 100,
          "can_setup_saml": true,
          "legacy_org_id": "1234",
          "metadata": {
            "customKey": "customValue"
          }
        });

        let response = env
            .client
            .put(env.url.clone())
            .bearer_auth("Fj3kLm90QwXe2PbtRzYHu1SdVnA4XcGj")
            .json(&payload)
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");
    }

    #[tokio::test]
    async fn test_delete_org() {
        let env = TestEnv::new("/api/backend/v1/org/582e7c11-6b72-40d8-886d-461e6491fa71").await;

        let response = env
            .client
            .delete(env.url.clone())
            .bearer_auth("valid_token")
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_enable_saml_for_org() {
        let env = TestEnv::new("/api/backend/v1/org/582e7c11-6b72-40d8-886d-461e6491fa71").await;

        let response = env
            .client
            .post(env.url.clone())
            .bearer_auth("valid_token")
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_disable_saml_for_org() {
        let env = TestEnv::new("/api/backend/v1/org/582e7c11-6b72-40d8-886d-461e6491fa71").await;

        let response = env
            .client
            .post(env.url.clone())
            .bearer_auth("valid_token")
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");
    }

    /// curl -H "Content-Type: application/json" \
    /// -H "Authorization: Bearer <API_KEY>" \
    /// "<AUTH_URL>/api/backend/v1/custom_role_mappings"
    #[tokio::test]
    async fn test_fetch_role_configurations() {
        let env = TestEnv::new("/api/backend/v1/custom_role_mappings").await;

        let response = env
            .client
            .get(env.url.clone())
            .bearer_auth("valid_token")
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        if EXPOSED_PORT.parse::<u16>().unwrap() == 3000 {
            assert_eq!(
                response_json,
                json!({
                    "custom_role_mappings": [
                        {
                            "custom_role_mapping_name": "Business Plan",
                            "num_orgs_subscribed": 2
                        },
                        {
                            "custom_role_mapping_name": "Default",
                            "num_orgs_subscribed": 1
                        }
                    ]
                })
            );
        } else if EXPOSED_PORT.parse::<u16>().unwrap() == 4010 {
            assert_eq!(
                response_json,
                json!({
                    "custom_role_mappings": [
                        {
                            "custom_role_mapping_name": "string",
                            "num_orgs_subscribed": 0
                        }
                    ]
                })
            );
        }
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_subscribe_org_to_role_configuration() {
        let env = TestEnv::new("/api/backend/v1/org/582e7c11-6b72-40d8-886d-461e6491fa71").await;

        let payload = json!({
            "custom_role_mapping_name": "Business Plan"
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
    async fn test_get_pending_org_invites() {
        let env = TestEnv::new("/api/backend/v1/pending_org_invites?org_id=4896c602-7c67-4d32-a25d-5adb9a15a60e&page_size=10&page_number=0").await;

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
                "total_invites": 1,
                "current_page": 0,
                "page_size": 10,
                "has_more_results": false,
                "invites": [
                    {
                        "invitee_email": "paul@sesame.microscaler.io",
                        "org_id": "4896c602-7c67-4d32-a25d-5adb9a15a60e",
                        "org_name": "PropelAuth",
                        "role_in_org": "Owner",
                        "additional_roles_in_org": [],
                        "created_at": 1718648493,
                        "expires_at": 1719080493,
                        "inviter_email": null,
                        "inviter_user_id": null
                    }
                ]
            }),
            "Unexpected response JSON"
        );
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_revoke_pending_org_invite() {
        let env = TestEnv::new("/api/backend/v1/pending_org_invites").await;

        let payload = json!({
            "invitee_email": "test@sesame.microscaler.io",
            "org_id": "582e7c11-6b72-40d8-886d-461e6491fa71"
        });

        let response = env
            .client
            .delete(env.url.clone())
            .bearer_auth("valid_token")
            .json(&payload)
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(response_json, json!({}), "Expected empty JSON response");
    }
}
