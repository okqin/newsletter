use crate::helpers::{get_confirmation_links, spawn_app, ConfirmationLinks, TestApp};
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn newsletter_are_not_delivered_to_unconfirmed_subscribers() {
    // Prepare
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    // Execute

    // The skeleton of the email newsletter payload structure
    // May change it later
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });
    let response = reqwest::Client::new()
        .post(format!("{}/newsletters", app.address))
        .json(&newsletter_request_body)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 200);
}

/// Use the public API of the program under test to create unconfirmed subscribers.
async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=na%20me&email=na_me%40example.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;
    app.post_subscriptions(body)
        .await
        .error_for_status()
        .unwrap();

    // Check the request received by the mock Postmark server to obtain the confirmation link and return it.
    let email_request = app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    get_confirmation_links(&email_request, app.app_port)
}
