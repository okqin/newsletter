use newsletter::{configuration::DatabaseSettings, HttpServer, Settings};
use reqwest::Client;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;

pub struct TestApp {
    pub address: String,
    pub app_port: u16,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub http_client: Client,
}

pub async fn spawn_app() -> TestApp {
    // start a mock email server
    let email_server = MockServer::start().await;

    // initialize an HTTP client
    let http_client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("Failed to build HTTP client");

    // randomize configuration to ensure test isolation
    let conf = {
        let mut c = Settings::try_load().expect("Failed to read config");
        // use a random OS port
        c.server.port = 0;
        // use a different database for each test case
        c.database.db_name = Uuid::new_v4().to_string();

        // use the mock email server
        c.email_client.base_url = email_server.uri();
        c
    };

    let db_pool = configure_database(&conf.database).await;

    let app = HttpServer::try_new(&conf).await.unwrap();
    let app_port = app.port();
    tokio::spawn(app.run());
    TestApp {
        address: format!("http://localhost:{}", app_port),
        app_port,
        db_pool,
        email_server,
        http_client,
    }
}

async fn configure_database(database: &DatabaseSettings) -> PgPool {
    // create database
    let database_settings = DatabaseSettings {
        db_name: "postgres".to_string(),
        ..database.clone()
    };
    let mut connection = PgConnection::connect(&database_settings.connection_string())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, database.db_name).as_ref())
        .await
        .expect("Failed to create database");

    // migrate database
    database.migrate_database().await.unwrap();
    database.get_connection_pool()
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: &str) -> reqwest::Response {
        self.http_client
            .post(format!("{}/subscriptions", self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body.to_string())
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub async fn get_subscriptions_confirm(&self, token: Option<&str>) -> reqwest::Response {
        let query = match token {
            Some(token) => format!("?subscription_token={}", token),
            None => String::new(),
        };
        self.http_client
            .get(format!("{}/subscriptions/confirm{}", self.address, query))
            .send()
            .await
            .expect("Failed to execute request")
    }
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub text: reqwest::Url,
}

pub fn get_confirmation_links(email_request: &wiremock::Request, port: u16) -> ConfirmationLinks {
    // convert the request body to a json value
    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

    // extract links from specific fields
    let get_link = |field: &str| {
        let links: Vec<_> = linkify::LinkFinder::new()
            .links(field)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(links.len(), 1);
        let raw_link = links[0].as_str();
        let mut confirmation_link = reqwest::Url::parse(raw_link).unwrap();
        assert_eq!(confirmation_link.host_str().unwrap(), "localhost");
        confirmation_link.set_port(Some(port)).unwrap();
        confirmation_link
    };

    let html_link = get_link(body["HtmlBody"].as_str().unwrap());
    let text_link = get_link(body["TextBody"].as_str().unwrap());
    ConfirmationLinks {
        html: html_link,
        text: text_link,
    }
}
