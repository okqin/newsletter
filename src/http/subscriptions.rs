use axum::{extract::State, http::StatusCode, routing::post, Form, Router};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use crate::http::{AppState, Result};

pub fn router() -> Router<AppState> {
    Router::new().route("/subscriptions", post(subscribe))
}

#[derive(Deserialize)]
struct FromData {
    email: String,
    name: String,
}

async fn subscribe(
    State(state): State<AppState>,
    Form(data): Form<FromData>,
) -> Result<StatusCode> {
    sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at) VALUES ($1, $2, $3, $4)"#,
        Uuid::new_v4(),
        data.email,
        data.name,
        Utc::now()
    )
    .execute(&state.db)
    .await?;
    Ok(StatusCode::OK)
}
