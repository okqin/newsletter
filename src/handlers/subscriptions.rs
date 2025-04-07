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

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    error::{ApiError, Result},
    router::{AppState, DbPool},
};

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
    let new_subscriber = match data.try_into() {
        Ok(subscriber) => subscriber,
        Err(e) => return ApiError::InvalidValue(e).into_response(),
    };
    match insert_subscriber(&state.db, &new_subscriber).await {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => e.into_response(),
    }
}

#[instrument(skip_all)]
async fn insert_subscriber(pool: &DbPool, new_subscriber: &NewSubscriber) -> Result<()> {
    sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at, status) VALUES ($1, $2, $3, $4, 'confirmed')"#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(pool)
    .await?;
    Ok(())
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, String> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(NewSubscriber { email, name })
    }
}
