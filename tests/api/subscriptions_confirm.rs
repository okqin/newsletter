use crate::helpers::{get_confirmation_links, spawn_app};
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
    // init
    let app = spawn_app().await;

    // execute
    let response = app.get_subscriptions_confirm(None).await;

    // assert
    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    // init
    let app = spawn_app().await;
    let body = "name=dhs%20doe&email=dhs_ni_hao%40example.com";

    // setup mock server
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // send subscription request
    app.post_subscriptions(body).await;

    // get the first request from the mock server
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    // extract links from the body
    let confirmation_link = get_confirmation_links(email_request, app.app_port);

    // execute
    let response = reqwest::get(confirmation_link.html).await.unwrap();

    // assert
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber() {
    // init
    let app = spawn_app().await;
    let body = "name=dhs%20doe&email=dhs_ni_hao%40example.com";

    // setup mock server
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_link = get_confirmation_links(email_request, app.app_port);

    // execute
    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    // assert
    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "dhs_ni_hao@example.com");
    assert_eq!(saved.name, "dhs doe");
    assert_eq!(saved.status, "confirmed");
}
