use axum::{http::StatusCode, routing::get, Router};

use crate::router::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/health_check", get(health_check))
}

async fn health_check() -> StatusCode {
    StatusCode::OK
}
