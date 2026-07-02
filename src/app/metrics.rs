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
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::extract::State;
use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use axum::routing::get;
use chrono::{DateTime, Utc};

use crate::dpc::{Dpc, DpcGrid, DpcProduct, Region};

const SAMPLE_PERIOD_SECS: i64 = 300;

struct Frame {
    time: DateTime<Utc>,
    poh: DpcGrid,
    vmi: DpcGrid,
    vil: DpcGrid,
    sri: DpcGrid,
    etm: DpcGrid,
}

pub struct Exporter {
    dpc: Dpc,
    region: Region,
    frame: Mutex<Option<Frame>>,
}

pub type Shared = Arc<Exporter>;

impl Exporter {
    pub fn new(dpc: Dpc, region: Region) -> Shared {
        Arc::new(Self {
            dpc,
            region,
            frame: Mutex::new(None),
        })
    }

    async fn refresh_if_stale(&self) {
        {
            let frame = self.frame.lock().unwrap();
            if let Some(f) = frame.as_ref() {
                if (Utc::now() - f.time).num_seconds() < SAMPLE_PERIOD_SECS {
                    return;
                }
            }
        }

        match self.fetch().await {
            Ok(frame) => {
                tracing::info!(time = %frame.time, "refreshed DPC frame");
                *self.frame.lock().unwrap() = Some(frame);
            }
            Err(e) => tracing::warn!(error = %e, "fetching DPC frame"),
        }
    }

    async fn fetch(&self) -> anyhow::Result<Frame> {
        let (poh, vmi, vil, sri, etm) = tokio::try_join!(
            self.dpc
                .fetch_latest_at(DpcProduct::ProbabilityOfHail, self.region),
            self.dpc
                .fetch_latest_at(DpcProduct::VerticalMaximumIntensity, self.region),
            self.dpc
                .fetch_latest_at(DpcProduct::VerticallyIntegratedLiquid, self.region),
            self.dpc
                .fetch_latest_at(DpcProduct::SurfaceRainfallIntensity, self.region),
            self.dpc
                .fetch_latest_at(DpcProduct::EchoTopMap, self.region),
        )?;

        Ok(Frame {
            time: poh.0,
            poh: poh.1,
            vmi: vmi.1,
            vil: vil.1,
            sri: sri.1,
            etm: etm.1,
        })
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

fn render(frame: &Frame) -> String {
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
