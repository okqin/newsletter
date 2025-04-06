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
        c.database.db_name = Uuid::new_v4().to_string();
        c
    };

    let db_pool = configure_database(&conf.database).await;

    let app = HttpServer::try_new(&conf).await.unwrap();
    let app_port = app.port();
    tokio::spawn(app.run());
    TestApp {
        address: format!("http://localhost:{}", app_port),
        db_pool,
    }
}

async fn configure_database(database: &DatabaseSettings) -> PgPool {
    // Create database
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

    // Migrate database
    database.migrate_database().await.unwrap();
    database.get_connection_pool()
}
