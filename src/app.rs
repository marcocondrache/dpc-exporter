use axum::{Router, http::StatusCode, routing::any};

use crate::dpc::Dpc;

mod health;
// mod metrics;

pub fn router(dpc: Dpc) -> Router {
    Router::new()
        .route("/", any(async || StatusCode::NO_CONTENT))
        .merge(health::router())
    // .merge(metrics::router(metrics::Exporter::new(dpc)))
}
