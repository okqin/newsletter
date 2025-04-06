use axum::http::HeaderName;
use std::sync::Arc;
use tower_http::{
    classify::{ServerErrorsAsFailures, SharedClassifier},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    sensitive_headers::{SetSensitiveRequestHeadersLayer, SetSensitiveResponseHeadersLayer},
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing::Level;

/// Returns a `TraceLayer` for HTTP requests and responses.
/// The `TraceLayer` is used to trace requests and responses in the application.
pub fn trace_layer() -> TraceLayer<SharedClassifier<ServerErrorsAsFailures>> {
    TraceLayer::new_for_http().make_span_with(
        DefaultMakeSpan::new()
            .level(Level::INFO)
            .include_headers(true),
    )
}

/// Add this layer before `TraceLayer` to hide sensitive request headers from log output
pub fn sensitive_request_headers(headers: Arc<[HeaderName]>) -> SetSensitiveRequestHeadersLayer {
    SetSensitiveRequestHeadersLayer::from_shared(headers)
}

/// Add this layer after `TraceLayer` to hide sensitive response headers from log output
pub fn sensitive_response_headers(headers: Arc<[HeaderName]>) -> SetSensitiveResponseHeadersLayer {
    SetSensitiveResponseHeadersLayer::from_shared(headers)
}

/// Add this layer before `TraceLayer` to set the request ID
pub fn set_x_request_id() -> SetRequestIdLayer<MakeRequestUuid> {
    SetRequestIdLayer::new(HeaderName::from_static("x-request-id"), MakeRequestUuid)
}

/// Add this layer before `TraceLayer` to propagate the request ID
pub fn propagate_x_request_id() -> PropagateRequestIdLayer {
    PropagateRequestIdLayer::new(HeaderName::from_static("x-request-id"))
}
