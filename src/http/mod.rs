use anyhow::Context;
use axum::{http::header, Router};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;

mod error;
mod health_check;
mod middleware;
mod subscriptions;

use crate::configuration::Settings;

pub use self::error::ApiError;

pub type Result<T, E = ApiError> = std::result::Result<T, E>;

pub type DbPool = sqlx::PgPool;

#[allow(dead_code)]
#[derive(Clone)]
pub(crate) struct AppState {
    pub config: Arc<Settings>,
    pub db: DbPool,
}

fn router(conf: &Settings) -> Router {
    let db = conf.database.get_connection_pool();
    let config = Arc::new(conf.clone());
    let app_state = AppState { config, db };

    let sensitive_headers = Arc::new([
        header::AUTHORIZATION,
        header::PROXY_AUTHORIZATION,
        header::COOKIE,
        header::SET_COOKIE,
    ]);

    let middleware = ServiceBuilder::new()
        .layer(middleware::set_x_request_id())
        .layer(middleware::sensitive_request_headers(
            sensitive_headers.clone(),
        ))
        .layer(middleware::trace_layer())
        .layer(middleware::sensitive_response_headers(sensitive_headers))
        .layer(middleware::propagate_x_request_id());

    Router::new()
        .merge(health_check::router())
        .merge(subscriptions::router())
        .layer(middleware)
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
    pub async fn try_new(conf: &Settings) -> anyhow::Result<Self> {
        let addr = conf.server.address_string();
        let listener = TcpListener::bind(addr)
            .await
            .context("Failed to bind address")?;
        let port = listener.local_addr().unwrap().port();
        let service = router(conf);
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
