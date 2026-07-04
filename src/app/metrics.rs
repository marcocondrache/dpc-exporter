//! Prometheus exporter for DPC hail products.
//!
//! There is no background task: `/metrics` refreshes lazily. DPC publishes a new
//! sample every 5 minutes, so the cached frame's own timestamp tells us whether
//! a fetch is even possible — while the cache is younger than one sample period
//! we serve it without touching DPC. Grafana does dashboards and alerting
//! (e.g. `dpc_hail_probability{scope="center"} > 0.5`).
//!
//! Data source: Radar-DPC (Dipartimento Protezione Civile), CC-BY-SA.

use std::fmt::Write;

use axum::Router;
use axum::extract::State;
use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use axum::routing::get;

use crate::{app::AppState, exporter::ExporterFrame};

pub fn router() -> Router<AppState> {
    Router::new().route("/metrics", get(handler))
}

async fn handler(State(state): State<AppState>) -> impl IntoResponse {
    let exp = &state.exporter;

    exp.refresh().await;
    let body = match &*exp.frame() {
        Some(frame) => render(frame),
        None => String::from("# no DPC frame fetched yet\n"),
    };

    ([(CONTENT_TYPE, "text/plain; version=0.0.4")], body)
}

fn render(frame: &ExporterFrame) -> String {
    let mut s = String::new();

    gauge(
        &mut s,
        "dpc_hail_probability",
        "Probability of hail (0-1), DPC POH.",
        frame.poh.center(),
        frame.poh.max(),
    );
    gauge(
        &mut s,
        "dpc_reflectivity_dbz",
        "Column-maximum reflectivity (dBZ), DPC VMI.",
        frame.vmi.center(),
        frame.vmi.max(),
    );
    gauge(
        &mut s,
        "dpc_vil_kgm2",
        "Vertically integrated liquid (kg/m^2), DPC VIL.",
        frame.vil.center(),
        frame.vil.max(),
    );
    gauge(
        &mut s,
        "dpc_rain_intensity_mmh",
        "Surface rainfall intensity (mm/h), DPC SRI.",
        frame.sri.center(),
        frame.sri.max(),
    );
    gauge(
        &mut s,
        "dpc_echo_top_meters",
        "Echo top height (m), DPC ETM.",
        frame.etm.center(),
        frame.etm.max(),
    );

    let _ = writeln!(
        s,
        "# HELP dpc_frame_timestamp_seconds Unix time of the DPC sample.\n\
         # TYPE dpc_frame_timestamp_seconds gauge\n\
         dpc_frame_timestamp_seconds {}",
        frame.time.timestamp()
    );
    s
}

fn gauge(s: &mut String, name: &str, help: &str, center: f32, area: f32) {
    let _ = writeln!(s, "# HELP {name} {help}");
    let _ = writeln!(s, "# TYPE {name} gauge");
    let _ = writeln!(s, "{name}{{scope=\"center\"}} {center}");
    let _ = writeln!(s, "{name}{{scope=\"area\"}} {area}");
}
