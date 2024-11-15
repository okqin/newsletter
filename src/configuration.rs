use anyhow::Context;
use config::{Config, ConfigError, File};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    PgPool,
};
use std::{path::PathBuf, time::Duration};

#[derive(Deserialize, Clone)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub server: ServerSettings,
    pub logs: Option<LogsSettings>,
}

#[derive(Deserialize, Clone)]
pub struct ServerSettings {
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
}

#[derive(Deserialize, Clone)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: SecretString,
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub database_name: String,
}

#[derive(Deserialize, Clone)]
pub struct LogsSettings {
    pub path: Option<PathBuf>,
    pub directives: Option<String>,
}

impl Settings {
    pub fn try_load() -> Result<Self, ConfigError> {
        // init config reader
        let settings = Config::builder()
            .add_source(File::with_name("config.toml"))
            .build()?;
        // try deserialize to Settings struct
        settings.try_deserialize::<Settings>()
    }
}

pub fn get_configuration() -> Result<Settings, ConfigError> {
    // init config reader
    let settings = Config::builder()
        .add_source(File::with_name("config.toml"))
        .build()?;
    // try deserialize to Settings struct
    settings.try_deserialize::<Settings>()
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name,
        )
    }

    // Manually-constructed options for PostgreSQL connection.
    pub fn connect_options(&self) -> PgConnectOptions {
        PgConnectOptions::new_without_pgpass()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .database(&self.database_name)
    }

    pub fn get_connection_pool(&self) -> PgPool {
        PgPoolOptions::new()
            .max_connections(1000)
            .max_lifetime(Duration::from_secs(60 * 60 * 24))
            .connect_lazy_with(self.connect_options())
    }

    pub async fn migrate_database(&self) -> anyhow::Result<()> {
        let connection_pool = PgPool::connect(&self.connection_string())
            .await
            .context("Failed to connect to Postgres")?;
        sqlx::migrate!("./migrations")
            .run(&connection_pool)
            .await
            .context("Failed to migrate the database")?;
        Ok(())
    }
}

impl ServerSettings {
    pub fn address_string(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
