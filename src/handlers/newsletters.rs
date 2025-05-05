use axum::{http::StatusCode, routing::post, Router};

use crate::router::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/newsletters", post(publish_newsletter))
}

async fn publish_newsletter() -> Result<StatusCode, ()> {
    Ok(StatusCode::OK)
}
