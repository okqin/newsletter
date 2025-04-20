use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    error::{ApiError, Result},
    router::{AppState, DbTransaction},
};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Form, Router,
};
use chrono::Utc;
use rand::{distr::Alphanumeric, rng, Rng};
use serde::Deserialize;
use tracing::instrument;
use uuid::Uuid;

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
    let mut transaction = match state.db.begin().await {
        Ok(transaction) => transaction,
        Err(e) => return ApiError::Database(e).into_response(),
    };
    let subscriber_id = match insert_subscriber(&mut transaction, &new_subscriber).await {
        Ok(subscriber_id) => subscriber_id,
        Err(e) => return e.into_response(),
    };

    let subscription_token = generate_subscription_token();
    if let Err(e) = store_token(&mut transaction, subscriber_id, &subscription_token).await {
        return e.into_response();
    }
    if let Err(e) = transaction.commit().await {
        return ApiError::Database(e).into_response();
    }
    if let Err(e) = send_confirmation_email(
        &state.email_client,
        new_subscriber,
        &state.base_url,
        &subscription_token,
    )
    .await
    {
        return e.into_response();
    }
    StatusCode::OK.into_response()
}

#[instrument(skip_all)]
async fn insert_subscriber(
    transaction: &mut DbTransaction<'_>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid> {
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
) -> Result<()> {
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
            new_subscriber.email,
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
) -> Result<()> {
    sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id) VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id
    )
    .execute(&mut **transaction)
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
