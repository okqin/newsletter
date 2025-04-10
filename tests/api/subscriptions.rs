use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_returns_200_valid_form_data() {
    // init
    let app = spawn_app().await;
    let body = "name=vic%20ji&email=vic_ji_i%40gmail.com";

    // execute
    let response = app.post_subscriptions(body).await;

    // assert
    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "vic_ji_i@gmail.com");
    assert_eq!(saved.name, "vic ji");
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
