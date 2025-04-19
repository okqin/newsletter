use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use serde::Deserialize;
use tracing::instrument;
use uuid::Uuid;

use crate::{
    error::Result,
    router::{AppState, DbPool},
};

pub fn router() -> Router<AppState> {
    Router::new().route("/subscriptions/confirm", get(confirm))
}

#[derive(Deserialize)]
struct Parameters {
    subscription_token: String,
}

#[instrument(name = "Confirm a pending subscriber", skip_all)]
pub async fn confirm(State(state): State<AppState>, parameters: Query<Parameters>) -> Response {
    let id = match get_subscriber_id_from_token(&state.db, &parameters.subscription_token).await {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };
    match id {
        None => StatusCode::UNAUTHORIZED.into_response(),
        Some(subscriber_id) => {
            if let Err(e) = confirm_subscriber(&state.db, subscriber_id).await {
                return e.into_response();
            }
            StatusCode::OK.into_response()
        }
    }
}

#[instrument(name = "Mark subscriber as confirmed", skip_all)]
pub async fn confirm_subscriber(pool: &DbPool, subscriber_id: Uuid) -> Result<()> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}

#[instrument(name = "Get subscriber_id from subscription_token", skip_all)]
pub async fn get_subscriber_id_from_token(
    pool: &DbPool,
    subscription_token: &str,
) -> Result<Option<Uuid>> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
        subscription_token
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(result.map(|r| r.subscriber_id))
}
