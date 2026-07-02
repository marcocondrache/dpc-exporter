use axum::{Router, http::StatusCode, routing::any};

use crate::dpc::{Dpc, Region};

mod health;
mod metrics;

pub fn router(dpc: Dpc, region: Region) -> Router {
    Router::new()
        .route("/", any(async || StatusCode::NO_CONTENT))
        .merge(health::router())
        .merge(metrics::router(metrics::Exporter::new(dpc, region)))
}
