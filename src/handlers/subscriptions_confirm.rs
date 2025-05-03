use anyhow::Context;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use tracing::{error, instrument, warn};
use uuid::Uuid;

use crate::{
    router::{AppState, DbPool, ErrorResponse},
    utils::error_chain_fmt,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/subscriptions/confirm", get(confirm))
}

#[derive(Deserialize)]
struct Parameters {
    subscription_token: String,
}

#[derive(thiserror::Error)]
pub enum ConfirmationError {
    #[error("There is no subscriber associated with the provided token.")]
    UnknownToken,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for ConfirmationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for ConfirmationError {
    fn into_response(self) -> Response {
        // Determine the appropriate status code.
        let status_code = match self {
            Self::UnknownToken => StatusCode::UNAUTHORIZED,
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        // Create the error response body.
        let body = ErrorResponse::new(status_code.as_u16(), self.to_string());

        // Log the error
        match self {
            Self::UnknownToken => warn!("{:?}", self),
            Self::UnexpectedError(e) => error!("{:?}", e),
        }

        (status_code, Json(body)).into_response()
    }
}

#[instrument(name = "Confirm a pending subscriber", skip_all)]
async fn confirm(
    State(state): State<AppState>,
    parameters: Query<Parameters>,
) -> Result<StatusCode, ConfirmationError> {
    let subscriber_id = get_subscriber_id_from_token(&state.db, &parameters.subscription_token)
        .await
        .context("Failed to retrieve the subscriber id associated with the provided token.")?
        .ok_or(ConfirmationError::UnknownToken)?;
    confirm_subscriber(&state.db, subscriber_id)
        .await
        .context("Failed to update the subscriber status to `confirmed`.")?;
    Ok(StatusCode::OK)
}

#[instrument(name = "Mark subscriber as confirmed", skip_all)]
async fn confirm_subscriber(pool: &DbPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[instrument(name = "Get subscriber_id from subscription_token", skip_all)]
async fn get_subscriber_id_from_token(
    pool: &DbPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
        subscription_token
    )
    .fetch_optional(pool)
    .await?;
    Ok(result.map(|r| r.subscriber_id))
}
