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
    email_client::EmailClient,
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
    let _ = match insert_subscriber(&state.db, &new_subscriber).await {
        Ok(_) => StatusCode::OK,
        Err(e) => return e.into_response(),
    };

    if let Err(e) = send_confirmation_email(&state.email_client, new_subscriber).await {
        return e.into_response();
    }
    StatusCode::OK.into_response()
}

#[instrument(skip_all)]
async fn insert_subscriber(pool: &DbPool, new_subscriber: &NewSubscriber) -> Result<()> {
    sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at, status) VALUES ($1, $2, $3, $4, 'pending_confirmation')"#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[instrument(name = "Send a confirmation email to a new subscriber", skip_all)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
) -> Result<()> {
    let confirmation_link = "http://my-api.com/subscriptions/confirm?email=";
    let html_content = format!(
        "Welcome to our newsletter! We're glad to have you.<br />\
        Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    let text_content = format!(
        "Welcome to our newsletter! We're glad to have you.\n\
        Visit {} to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email(
            new_subscriber.email,
            "Welcome!",
            &html_content,
            &text_content,
        )
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
