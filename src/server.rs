use anyhow::Context;
use axum::Router;
use std::sync::Arc;
use tokio::net::TcpListener;

use crate::{
    configuration::Settings,
    email_client::EmailClient,
    router::{build_router, AppState},
};

/// HTTP Server wrapper to facilitate integration testing and service initialization
pub struct HttpServer {
    listener: TcpListener,
    port: u16,
    service: Router,
}

impl HttpServer {
    /// Creates a new HTTP server
    ///
    /// # Arguments
    /// * `conf` - Application settings
    ///
    /// # Returns
    /// A new HttpServer instance if successful, Error otherwise
    pub async fn try_new(conf: &Settings) -> anyhow::Result<Self> {
        let addr = conf.server.address_string();
        let listener = TcpListener::bind(addr)
            .await
            .context("Failed to bind address")?;

        let port = listener
            .local_addr()
            .context("Failed to get local address")?
            .port();

        // Builds the application state from configuration settings
        let app_state = build_app_state(conf);

        // Build router with app state
        let service = build_router(app_state);

        Ok(Self {
            listener,
            port,
            service,
        })
    }

    /// Starts the HTTP server
    ///
    /// # Returns
    /// Ok(()) if successful, Error otherwise
    pub async fn run(self) -> anyhow::Result<()> {
        axum::serve(self.listener, self.service)
            .await
            .context("Failed to start server")
    }

    /// Returns the port the server is listening on
    pub fn port(&self) -> u16 {
        self.port
    }
}

fn build_app_state(conf: &Settings) -> AppState {
    // Create database connection pool from configuration
    let db = conf.database.get_connection_pool();

    // Parse sender email from configuration
    let sender_email = conf
        .email_client
        .sender()
        .expect("Invalid sender email address");

    // Create new email client with configuration parameters
    let email_client = EmailClient::new(
        &conf.email_client.base_url,
        sender_email,
        conf.email_client.authorization_token.clone(),
        conf.email_client.timeout(),
    );
    // Wrap email client in Arc for thread-safe sharing
    let email_client = Arc::new(email_client);

    // Get base URL from configuration
    let base_url = Arc::new(conf.server.base_url.clone());

    // Return the application state with all components
    AppState {
        db,
        email_client,
        base_url,
    }
}
