use crate::helpers::{get_confirmation_links, spawn_app};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn subscribe_returns_200_valid_form_data() {
    // init
    let app = spawn_app().await;
    let body = "name=vic%20ji&email=vic_ji_i%40gmail.com";

    // setup mock server
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // execute
    let response = app.post_subscriptions(body).await;

    // assert
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    // init
    let app = spawn_app().await;
    let body = "name=vic%20ji&email=vic_ji_i%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // execute
    app.post_subscriptions(body).await;

    // assert
    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "vic_ji_i@gmail.com");
    assert_eq!(saved.name, "vic ji");
    assert_eq!(saved.status, "pending_confirmation");
}

#[tokio::test]
async fn subscribe_returns_422_when_data_is_missing() {
    // init
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=vic%20ji", "missing the email"),
        ("email=vic_ji%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // execute
        let response = app.post_subscriptions(invalid_body).await;

        // assert
        assert_eq!(
            422,
            response.status().as_u16(),
            "The API did not fail with 422 Unprocessable Entity when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_400_when_fields_are_present_but_invalid() {
    // init
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=&email=vic_ji%40gmail.com", "name is empty"),
        ("name=vic%20ji&email=", "email is empty"),
        ("name=vic%20ji&email=a-invalid-email", "invalid email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // execute
        let response = app.post_subscriptions(invalid_body).await;

        // assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    // init
    let app = spawn_app().await;
    let body = "name=vic%20ji&email=vic_ji_i%40gmail.com";

    // setup mock server
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // execute
    app.post_subscriptions(body).await;

    // assert
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    // init
    let app = spawn_app().await;
    let body = "name=vic%20ji&email=vic_ji_i%40gmail.com";

    // setup mock server
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // execute
    app.post_subscriptions(body).await;

    // assert
    // get the first request
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    // extract links from the body
    let confirmation_links = get_confirmation_links(email_request, app.app_port);

    // two links should be the same.
    assert_eq!(confirmation_links.html, confirmation_links.text);
}
