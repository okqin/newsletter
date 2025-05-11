use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    router::{AppState, DbTransaction, ErrorResponse},
    utils::error_chain_fmt,
};
use anyhow::Context;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Form, Json, Router,
};
use chrono::Utc;
use rand::{distr::Alphanumeric, rng, Rng};
use serde::Deserialize;
use tracing::{error, instrument, warn};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new().route("/subscriptions", post(subscribe))
}

#[derive(Deserialize)]
struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, String> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(NewSubscriber { email, name })
    }
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for SubscribeError {
    #[instrument(skip_all)]
    fn into_response(self) -> Response {
        // Determine the appropriate status code.
        let status_code = match self {
            Self::ValidationError(_) => StatusCode::BAD_REQUEST,
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        // Create the error response body
        let body = ErrorResponse::new(status_code.as_u16(), self.to_string());

        // Log the error
        match self {
            Self::ValidationError(e) => warn!("{:?}", e),
            Self::UnexpectedError(e) => error!("{:?}", e),
        }

        (status_code, Json(body)).into_response()
    }
}

#[instrument(name = "Add a new subscriber" , skip_all, fields(subscriber_email = data.email, subscriber_name = data.name))]
pub async fn subscribe(
    State(state): State<AppState>,
    Form(data): Form<FormData>,
) -> Result<StatusCode, SubscribeError> {
    let new_subscriber = data.try_into().map_err(SubscribeError::ValidationError)?;

    let mut transaction = state
        .db
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool.")?;
    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .context("Failed to insert new subscriber in the database.")?;
    let subscription_token = generate_subscription_token();
    store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .context("Failed to store the confirmation token for a new subscriber.")?;
    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new subscriber.")?;
    send_confirmation_email(
        &state.email_client,
        new_subscriber,
        &state.base_url,
        &subscription_token,
    )
    .await
    .context("Failed to send a confirmation email.")?;
    Ok(StatusCode::OK)
}

#[instrument(name = "Save new subscriber details in the database", skip_all)]
async fn insert_subscriber(
    transaction: &mut DbTransaction<'_>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at, status) VALUES ($1, $2, $3, $4, 'pending_confirmation')"#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(&mut **transaction)
    .await?;
    Ok(subscriber_id)
}

#[instrument(name = "Send a confirmation email to a new subscriber", skip_all)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );
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
            &new_subscriber.email,
            "Welcome!",
            &html_content,
            &text_content,
        )
        .await?;
    Ok(())
}

fn generate_subscription_token() -> String {
    let mut rng = rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

#[instrument(name = "Store subscription token in the database", skip_all)]
async fn store_token(
    transaction: &mut DbTransaction<'_>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id) VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
