use newsletter::{configuration::DatabaseSettings, HttpServer, Settings};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

pub async fn spawn_app() -> TestApp {
    // Randomize configuration to ensure test isolation
    let conf = {
        let mut c = Settings::try_load().expect("Failed to read config");
        // Use a random OS port
        c.server.port = 0;
        // Use a different database for each test case
        c.database.database_name = Uuid::new_v4().to_string();
        c
    };

    // Create database
    let create_database_settings = DatabaseSettings {
        database_name: "postgres".to_string(),
        ..conf.database.clone()
    };
    let mut connection = PgConnection::connect(&create_database_settings.connection_string())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, conf.database.database_name).as_ref())
        .await
        .expect("Failed to create database");

    // Migrate database
    conf.database.migrate_database().await.unwrap();

    let app = HttpServer::try_new(&conf).await.unwrap();
    let app_port = app.port();
    tokio::spawn(app.serve());
    TestApp {
        address: format!("http://localhost:{}", app_port),
        db_pool: conf.database.get_connection_pool(),
    }
}

#[tokio::test]
async fn health_check_works() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/health_check", app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_200_valid_form_data() {
    // init
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // execute
    let body = "name=vic%20ji&email=vic_ji_i%40gmail.com";
    let response = client
        .post(format!("{}/subscriptions", app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

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
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=vic%20ji", "missing the email"),
        ("email=vic_ji%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // execute
        let response = client
            .post(format!("{}/subscriptions", app.address))
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
