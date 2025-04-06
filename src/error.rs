use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;
use tracing::{instrument, warn};

/// Result type specific to the HTTP API
pub type Result<T, E = ApiError> = std::result::Result<T, E>;

/// Custom error type for the API.
/// The `#[from]` attribute allows for easy conversion from other error types.
#[derive(Error, Debug)]
pub enum ApiError {
    /// Converts from `sqlx::Error`.
    #[error("a database error has occurred")]
    Database(#[from] sqlx::Error),

    /// Converts from any `anyhow::Error`.
    #[error("an internal server error has occurred")]
    Internal(#[from] anyhow::Error),

    /// Invalid value error.
    #[error("invalid value: {0}")]
    InvalidValue(String),
}

// Provide detailed error messages as needed
#[derive(Serialize)]
struct ErrorResponse {
    message: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<String>,
}

impl ApiError {
    // Determine the appropriate status code.
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Internal(_) | Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::InvalidValue(_) => StatusCode::BAD_REQUEST,
        }
    }

    // Get the wrapped error message inside ApiError
    fn inner_error(&self) -> String {
        match self {
            Self::Internal(e) => format!("internal error: {}", e),
            Self::Database(e) => format!("database error: {}", e),
            Self::InvalidValue(e) => format!("invalid value: {}", e),
        }
    }
}

// The IntoResponse implementation for Api Error
// Create a generic response to hide specific implementation details.
impl IntoResponse for ApiError {
    #[instrument(skip_all)]
    fn into_response(self) -> Response {
        let status_code = self.status_code();

        let body = ErrorResponse {
            message: self.to_string(),
            errors: None,
        };

        let error_log = self.inner_error();
        warn!(error_log);

        (status_code, Json(body)).into_response()
    }
}
