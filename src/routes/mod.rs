mod health_check;
mod subscriptions;

use axum::{
    routing::{get, post},
    Router,
};

pub use health_check::*;
pub use subscriptions::*;

pub fn router() -> Router {
    Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
}
