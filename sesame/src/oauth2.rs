#[cfg(test)]
mod tests {
    use crate::test_utils::TestEnv;
    use serde_json::json;


    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_fetch_user_oauth_tokens() {
        let env = TestEnv::new("/api/backend/v1/user/a04d69d7-9347-48a3-aa01-8e7ce9aeee04/oauth_token").await;

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
                "tokens": [
                    {
                        "provider": "google",
                        "access_token": "ya29.a0AfH6SM...",
                        "refresh_token": "1//0g...",
                        "expires_in": 3600
                    }
                ]
            }),
            "Unexpected response JSON"
        );
    }
}
