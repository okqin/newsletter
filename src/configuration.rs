use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub server_port: u16,
}

#[derive(Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
    pub port: u16,
    pub database_name: String,
}

pub fn get_configuration() -> Result<Settings, ConfigError> {
    // init config reader
    let settings = Config::builder()
        .add_source(File::with_name("config.toml"))
        .build()?;
    // try deserialize to Settings struct
    settings.try_deserialize::<Settings>()
}
