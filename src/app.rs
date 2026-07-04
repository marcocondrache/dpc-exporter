use std::sync::Arc;

use axum::{Router, http::StatusCode, routing::any};

use crate::exporter::Exporter;

mod health;
mod metrics;

#[derive(Clone)]
pub struct AppState {
    pub exporter: Arc<Exporter>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", any(async || StatusCode::NO_CONTENT))
        .merge(health::router())
        .merge(metrics::router())
        .with_state(state)
}
