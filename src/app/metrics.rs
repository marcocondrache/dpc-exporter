//! Prometheus exporter for DPC hail products.
//!
//! There is no background task: `/metrics` refreshes lazily. DPC publishes a new
//! sample every 5 minutes, so the cached frame's own timestamp tells us whether
//! a fetch is even possible — while the cache is younger than one sample period
//! we serve it without touching DPC. Grafana does dashboards and alerting
//! (e.g. `hailwarden_hail_probability{scope="home"} > 0.5`).
//!
//! Data source: Radar-DPC (Dipartimento Protezione Civile), CC-BY-SA.

use std::fmt::Write;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::extract::State;
use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use axum::routing::get;
use chrono::Utc;

use crate::dpc::Dpc;

/// DPC's sample cadence in seconds (products are published every 5 minutes).
const SAMPLE_PERIOD_SECS: i64 = 300;

/// Holds the DPC client and the last frame; refreshed on scrape when stale.
pub struct Exporter {
    dpc: Dpc,
    frame: Mutex<Option<HailFrame>>,
}

pub type Shared = Arc<Exporter>;

impl Exporter {
    pub fn new(dpc: Dpc) -> Shared {
        Arc::new(Self {
            dpc,
            frame: Mutex::new(None),
        })
    }

    /// Fetch a new frame if the cache is empty or a newer sample may exist.
    async fn refresh_if_stale(&self) {
        // Read the cached sample time and drop the guard before awaiting.
        let known_ms = {
            let frame = self.frame.lock().unwrap();
            match frame.as_ref() {
                // Within one sample period no newer sample can exist yet.
                Some(f) if (Utc::now() - f.time).num_seconds() < SAMPLE_PERIOD_SECS => return,
                Some(f) => Some(f.time.timestamp_millis()),
                None => None,
            }
        };

        match self.dpc.latest_sample_time().await {
            Ok(t) if Some(t) != known_ms => match self.dpc.fetch_at(t).await {
                // ponytail: two concurrent stale scrapes may both fetch; the
                // result is identical, last write wins. Fine for a home exporter.
                Ok(frame) => {
                    tracing::info!(time = %frame.time, "refreshed DPC frame");
                    *self.frame.lock().unwrap() = Some(frame);
                }
                Err(e) => tracing::warn!(error = %e, "fetching DPC frame"),
            },
            Ok(_) => {} // already have the latest sample.
            Err(e) => tracing::warn!(error = %e, "polling DPC sample time"),
        }
    }
}

pub fn router(state: Shared) -> Router {
    Router::new()
        .route("/metrics", get(handler))
        .with_state(state)
}

async fn handler(State(exp): State<Shared>) -> impl IntoResponse {
    exp.refresh_if_stale().await;
    let body = match &*exp.frame.lock().unwrap() {
        Some(frame) => render(frame),
        None => String::from("# no DPC frame fetched yet\n"),
    };
    ([(CONTENT_TYPE, "text/plain; version=0.0.4")], body)
}

fn render(frame: &HailFrame) -> String {
    let (hr, hc) = frame.home;
    let mut s = String::new();

    gauge(
        &mut s,
        "hailwarden_hail_probability",
        "Probability of hail (0-1), DPC POH.",
        frame.poh[hr][hc],
        area_max(&frame.poh),
    );
    gauge(
        &mut s,
        "hailwarden_reflectivity_dbz",
        "Column-maximum reflectivity (dBZ), DPC VMI.",
        frame.vmi[hr][hc],
        area_max(&frame.vmi),
    );
    gauge(
        &mut s,
        "hailwarden_vil_kgm2",
        "Vertically integrated liquid (kg/m^2), DPC VIL.",
        frame.vil[hr][hc],
        area_max(&frame.vil),
    );

    let _ = writeln!(
        s,
        "# HELP hailwarden_frame_timestamp_seconds Unix time of the DPC sample.\n\
         # TYPE hailwarden_frame_timestamp_seconds gauge\n\
         hailwarden_frame_timestamp_seconds {}",
        frame.time.timestamp()
    );
    s
}

/// Write one gauge with `home` and `area` series.
fn gauge(s: &mut String, name: &str, help: &str, home: f32, area: f32) {
    let _ = writeln!(s, "# HELP {name} {help}");
    let _ = writeln!(s, "# TYPE {name} gauge");
    let _ = writeln!(s, "{name}{{scope=\"home\"}} {home}");
    let _ = writeln!(s, "{name}{{scope=\"area\"}} {area}");
}

/// Maximum over a grid, ignoring `NaN` (no-data) cells; `NaN` if all missing.
fn area_max(grid: &[Vec<f32>]) -> f32 {
    grid.iter()
        .flatten()
        .copied()
        .filter(|v| !v.is_nan())
        .fold(f32::NAN, f32::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> HailFrame {
        HailFrame {
            time: Utc::now(),
            home: (0, 0),
            step_m: 1000.0,
            poh: vec![vec![0.2, 0.8], vec![f32::NAN, 0.1]],
            vil: vec![vec![5.0, 9.0], vec![1.0, 2.0]],
            vmi: vec![vec![30.0, 55.0], vec![10.0, 20.0]],
        }
    }

    #[test]
    fn area_max_ignores_nan() {
        assert_eq!(area_max(&frame().poh), 0.8);
        assert!(area_max(&[vec![f32::NAN]]).is_nan());
    }

    #[test]
    fn render_emits_home_and_area_series() {
        let out = render(&frame());
        assert!(out.contains("hailwarden_hail_probability{scope=\"home\"} 0.2"));
        assert!(out.contains("hailwarden_hail_probability{scope=\"area\"} 0.8"));
        assert!(out.contains("hailwarden_reflectivity_dbz{scope=\"area\"} 55"));
        assert!(out.contains("# TYPE hailwarden_vil_kgm2 gauge"));
        assert!(out.contains("hailwarden_frame_timestamp_seconds "));
    }
}
