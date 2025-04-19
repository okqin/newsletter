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

/// Main application settings structure
#[derive(Deserialize, Clone)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub server: ServerSettings,
    pub logs: Option<LogsSettings>,
    pub email_client: EmailClientSettings,
}

/// HTTP server configuration settings
#[derive(Deserialize, Clone)]
pub struct ServerSettings {
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub base_url: String,
}

/// Database connection settings
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

/// Logging configuration settings
#[derive(Deserialize, Clone)]
pub struct LogsSettings {
    pub path: Option<PathBuf>,
    pub directives: Option<String>,
}

/// Email client configuration settings
#[derive(Deserialize, Clone)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    pub authorization_token: SecretString,
    pub timeout_millis: u64,
}

impl Settings {
    /// Attempts to load settings from configuration files and environment variables
    ///
    /// # Returns
    /// Configuration settings if successful, ConfigError otherwise
    ///
    /// # Environment Variables
    /// - APP_ENVIRONMENT: "dev" or "production" (defaults to "dev")
    pub fn try_load() -> Result<Self, ConfigError> {
        let base_path = std::env::current_dir().expect("Failed to determine current directory");
        let config_dir = base_path.join("config");

        // Detect the running environment.
        // Default to `dev` if unspecified.
        let environment: Environment = std::env::var("APP_ENVIRONMENT")
            .unwrap_or_else(|_| "dev".into())
            .try_into()
            .expect("Failed to parse APP_ENVIRONMENT");

        let environment_name = environment.as_str();
        let config_file = format!("{}.toml", environment_name);

        // Initialize configuration reader
        let settings = Config::builder()
            .add_source(File::from(config_dir.join("base.toml")))
            .add_source(File::from(config_dir.join(config_file)))
            // Add settings from environment variables (with a prefix of APP and '.' as separator)
            // E.g. `APP.SERVER.PORT=8000 would set `Settings.server.port`
            .add_source(
                config::Environment::with_prefix("APP")
                    .prefix_separator(".")
                    .separator("."),
            )
            .build()?;

        // Deserialize to Settings struct
        settings.try_deserialize::<Settings>()
    }
}

impl DatabaseSettings {
    /// Returns a connection string for this database
    ///
    /// # Returns
    /// A PostgreSQL connection string
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

    /// Returns connection options for this database
    ///
    /// # Returns
    /// PostgreSQL connection options
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

    /// Creates a connection pool for this database
    ///
    /// # Returns
    /// A PostgreSQL connection pool
    pub fn get_connection_pool(&self) -> PgPool {
        PgPoolOptions::new()
            .acquire_timeout(Duration::from_secs(5))
            .max_connections(1000)
            .max_lifetime(Duration::from_secs(60 * 60 * 24))
            .connect_lazy_with(self.connect_options())
    }

    /// Migrates the database schema
    ///
    /// # Returns
    /// Ok(()) if successful, Error otherwise
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
    /// Returns the address string in the format "host:port"
    pub fn address_string(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

impl EmailClientSettings {
    /// Parses the sender email address
    ///
    /// # Returns
    /// A valid SubscriberEmail if successful, Error otherwise
    pub fn sender(&self) -> Result<SubscriberEmail, String> {
        SubscriberEmail::parse(self.sender_email.clone())
    }

    /// Returns the timeout duration
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_millis)
    }
}

/// The possible runtime environment for our application.
pub enum Environment {
    Dev,
    Production,
}

impl Environment {
    /// Returns the environment name as a string
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
                "{} is not a supported environment. Use either `dev` or `production`.",
                other
            )),
        }
    }
}
