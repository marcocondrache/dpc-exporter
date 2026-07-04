use axum::{Router, http::StatusCode, routing::get};

use super::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/health", get(health))
}

async fn health() -> StatusCode {
    StatusCode::NO_CONTENT
}
