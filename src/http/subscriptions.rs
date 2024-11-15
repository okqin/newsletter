use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Form, Router,
};
use chrono::Utc;
use serde::Deserialize;
use tracing::instrument;
use uuid::Uuid;

use crate::http::{AppState, DbPool, Result};

pub fn router() -> Router<AppState> {
    Router::new().route("/subscriptions", post(subscribe))
}

#[derive(Deserialize)]
struct FormData {
    email: String,
    name: String,
}

#[instrument(skip_all, fields(subscriber_email = data.email, subscriber_name = data.name))]
async fn subscribe(State(state): State<AppState>, Form(data): Form<FormData>) -> Response {
    match insert_subscriber(&state.db, &data).await {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => e.into_response(),
    }
}

#[instrument(skip_all)]
async fn insert_subscriber(pool: &DbPool, form: &FormData) -> Result<()> {
    sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at) VALUES ($1, $2, $3, $4)"#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(pool)
    .await?;
    Ok(())
}
