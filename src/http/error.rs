use axum::{
    extract::rejection::FormRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

/// Custom error type for the API.
/// The `#[from]` attribute allows for easy conversion from other error types.
#[derive(Error, Debug)]
pub enum Error {
    /// Converts from an Axum built-in extractor error.
    #[error("Invalid payload")]
    InvalidFormBody(#[from] FormRejection),

    /// For errors that occur during manual validation.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Converts from `sqlx::Error`.
    #[error("A database error has occurred")]
    DatabaseError(#[from] sqlx::Error),

    /// Converts from any `anyhow::Error`.
    #[error("An internal server error has occurred")]
    InternalError(#[from] anyhow::Error),
}

// Provide detailed error messages as needed
#[derive(Serialize)]
struct ErrorResponse {
    message: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<String>,
}

impl Error {
    // Determine the appropriate status code.
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidFormBody(e) => e.status(),
            Self::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            Self::InternalError(_) | Self::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

// The IntoResponse implementation for Api Error
// Create a generic response to hide specific implementation details.
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status_code = self.status_code();

        let errors = match &self {
            Self::InvalidFormBody(errors) => Some(errors.body_text()),
            _ => None,
        };

        let body = ErrorResponse {
            message: self.to_string(),
            errors,
        };
        (status_code, Json(body)).into_response()
    }
}
