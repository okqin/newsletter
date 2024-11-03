use axum::{http::StatusCode, response::IntoResponse, Form};
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct FromData {
    email: String,
    name: String,
}

pub async fn subscribe(Form(_data): Form<FromData>) -> impl IntoResponse {
    StatusCode::OK
}
