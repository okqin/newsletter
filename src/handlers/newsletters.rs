use anyhow::Context;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use tracing::{error, instrument, warn};

use crate::{
    domain::SubscriberEmail,
    router::{AppState, DbPool, ErrorResponse},
    utils::error_chain_fmt,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/newsletters", post(publish_newsletter))
}

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    text: String,
    html: String,
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for PublishError {
    #[instrument(skip_all)]
    fn into_response(self) -> Response {
        // Determine the appropriate status code.
        let status_code = match self {
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        // Create the error response body
        let body = ErrorResponse::new(status_code.as_u16(), self.to_string());

        // Log the error
        match self {
            Self::UnexpectedError(e) => error!("{:?}", e),
        }

        (status_code, Json(body)).into_response()
    }
}

pub async fn publish_newsletter(
    State(state): State<AppState>,
    Json(body): Json<BodyData>,
) -> Result<StatusCode, PublishError> {
    let subscribers = get_confirmed_subscribers(&state.db).await?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                state
                    .email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })?;
            }
            Err(error) => {
                warn!(
                    error.cause_chain = ?error,
                    "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid"
                );
            }
        }
    }
    Ok(StatusCode::OK)
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[instrument(name = "Get confirmed subscribers", skip_all)]
async fn get_confirmed_subscribers(
    pool: &DbPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers =
        sqlx::query!(r#"SELECT email FROM subscriptions WHERE status = 'confirmed'"#)
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|r| match SubscriberEmail::parse(r.email) {
                Ok(email) => Ok(ConfirmedSubscriber { email }),
                Err(e) => Err(anyhow::anyhow!(e)),
            })
            .collect();
    Ok(confirmed_subscribers)
}
