/// Start a application instance
/// and then return the url (like: http://localhost:xxxx)
async fn spawn_app() -> String {
    let addr = "0.0.0.0:0";
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind address");
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(newsletter::startup::run(listener));
    format!("http://127.0.0.1:{}", port)
}

#[tokio::test]
async fn health_check_works() {
    let app_address = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/health_check", app_address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_200_valid_form_data() {
    // init
    let app_address = spawn_app().await;
    let client = reqwest::Client::new();

    // execute
    let body = "name=vic%20ji&email=vic_ji%40gmail.com";
    let response = client
        .post(format!("{}/subscriptions", app_address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    // assert
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_returns_422_when_data_is_missing() {
    // init
    let app_address = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=vic%20ji", "missing the email"),
        ("email=vic_ji%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // execute
        let response = client
            .post(format!("{}/subscriptions", app_address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        // assert
        assert_eq!(
            422,
            response.status().as_u16(),
            "The API did not fail with 422 Unprocessable Entity when the payload was {}.",
            error_message
        );
    }
}
