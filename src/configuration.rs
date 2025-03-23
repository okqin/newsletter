use anyhow::Context;
use config::{Config, ConfigError, File};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
    PgPool,
};
use std::{path::PathBuf, time::Duration};

use crate::domain::SubscriberEmail;

#[derive(Deserialize, Clone)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub server: ServerSettings,
    pub logs: Option<LogsSettings>,
    pub email_client: EmailClientSettings,
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
    pub db_name: String,
    pub require_ssl: bool,
}

#[derive(Deserialize, Clone)]
pub struct LogsSettings {
    pub path: Option<PathBuf>,
    pub directives: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    pub authorization_token: SecretString,
}

impl Settings {
    pub fn try_load() -> Result<Self, ConfigError> {
        let base_path = std::env::current_dir().expect("Failed to get current directory");
        let config_dir = base_path.join("config");

        // Detect the running environment.
        // Default to `local` if unspecified.
        let environment: Environment = std::env::var("APP_ENVIRONMENT")
            .unwrap_or_else(|_| "dev".into())
            .try_into()
            .expect("Failed to parse APP_ENVIRONMENT.");
        let config_file = format!("{}.toml", environment.as_str());

        // init config reader
        let settings = Config::builder()
            .add_source(File::from(config_dir.join("base.toml")))
            .add_source(File::from(config_dir.join(config_file)))
            // Add in settings from environment variables (with a prefix of APP and '.' as separator)
            // E.g. `APP.SERVER.PORT=8000 would set `Settings.server.port`
            .add_source(
                config::Environment::with_prefix("APP")
                    .prefix_separator(".")
                    .separator("."),
            )
            .build()?;
        // try deserialize to Settings struct
        settings.try_deserialize::<Settings>()
    }
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.db_name,
        )
    }

    // Manually-constructed options for PostgreSQL connection.
    pub fn connect_options(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };

        PgConnectOptions::new_without_pgpass()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .database(&self.db_name)
            .ssl_mode(ssl_mode)
    }

    pub fn get_connection_pool(&self) -> PgPool {
        PgPoolOptions::new()
            .acquire_timeout(Duration::from_secs(5))
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

impl EmailClientSettings {
    pub fn sender(&self) -> Result<SubscriberEmail, String> {
        SubscriberEmail::parse(self.sender_email.clone())
    }
}

/// The possible runtime environment for our application.
pub enum Environment {
    Dev,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "dev" => Ok(Self::Dev),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. Use either `local` or `production`.",
                other
            )),
        }
    }
}
