use axum::{http::header, Router};
use std::sync::Arc;
use tower::ServiceBuilder;

use crate::{
    email_client::EmailClient,
    handlers::{health_check, newsletters, subscriptions, subscriptions_confirm},
    middleware,
};

/// Postgres database pool type
pub type DbPool = sqlx::PgPool;

/// Postgres database transaction type
pub type DbTransaction<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

/// Error response body structure
#[derive(serde::Serialize)]
pub struct ErrorResponse {
    code: u16,
    message: String,
}

impl ErrorResponse {
    /// Creates a new error response body
    pub fn new(code: u16, message: String) -> Self {
        Self { code, message }
    }
}

/// Application state shared across all routes
#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct AppState {
    pub db: DbPool,
    pub email_client: Arc<EmailClient>,
    pub base_url: Arc<String>,
}

/// Builds the API router with all routes and middlewares
///
/// # Arguments
/// * `app_state` - Application state
///
/// # Returns
/// A configured router with all routes and middleware
pub(crate) fn build_router(app_state: AppState) -> Router {
    // Configure headers that should be treated as sensitive in logs
    let sensitive_headers = Arc::new([
        header::AUTHORIZATION,
        header::PROXY_AUTHORIZATION,
        header::COOKIE,
        header::SET_COOKIE,
    ]);

    // Build middleware stack
    let middleware = ServiceBuilder::new()
        .layer(middleware::set_x_request_id())
        .layer(middleware::sensitive_request_headers(
            sensitive_headers.clone(),
        ))
        .layer(middleware::trace_layer())
        .layer(middleware::sensitive_response_headers(sensitive_headers))
        .layer(middleware::propagate_x_request_id());

    // Create router with all routes and middleware
    Router::new()
        .merge(health_check::router())
        .merge(subscriptions::router())
        .merge(subscriptions_confirm::router())
        .merge(newsletters::router())
        .layer(middleware)
        .with_state(app_state)
}
