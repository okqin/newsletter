use anyhow::Context;
use axum::Router;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::net::TcpListener;

mod error;
mod health_check;
mod subscriptions;

use crate::configuration::Settings;

pub use self::error::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[allow(dead_code)]
#[derive(Clone)]
pub(crate) struct AppState {
    pub config: Arc<Settings>,
    pub db: PgPool,
}

fn app(conf: Settings) -> Router {
    let db = conf.database.get_connection_pool();
    let config = Arc::new(conf);
    let app_state = AppState { config, db };
    Router::new()
        .merge(health_check::router())
        .merge(subscriptions::router())
        .with_state(app_state)
}

/// To facilitate integrated testing, wrap up the construction of services.
/// The `port` field is used to store the actual port number when a random port is used for testing.
pub struct HttpServer {
    listener: TcpListener,
    port: u16,
    service: Router,
}

impl HttpServer {
    /// Initialize the Listener and Router, and then start the service through the serve method.
    pub async fn new(conf: Settings) -> anyhow::Result<Self> {
        let addr = conf.server.address_string();
        let listener = TcpListener::bind(addr)
            .await
            .context("Failed to bind address")?;
        let port = listener.local_addr().unwrap().port();
        let service = app(conf);
        Ok(Self {
            listener,
            port,
            service,
        })
    }

    /// Start the service.
    pub async fn serve(self) -> anyhow::Result<()> {
        axum::serve(self.listener, self.service)
            .await
            .context("Failed to start server")
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}
