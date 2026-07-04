mod app;
mod dpc;
mod exporter;
mod server;
mod telemetry;

use std::{net::SocketAddr, sync::Arc};

use clap::Parser;

use crate::dpc::{DpcClient, Region};

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(long, env = "BIND", default_value = "0.0.0.0:8080")]
    bind: SocketAddr,

    #[arg(long, env = "RUST_LOG", default_value = "info")]
    log: String,

    /// latitude (WGS84).
    #[arg(long, env = "LAT")]
    lat: f64,

    /// longitude (WGS84).
    #[arg(long, env = "LON")]
    lon: f64,

    /// Radius around to monitor, in km.
    #[arg(long, env = "RADIUS_KM", default_value_t = 20.0)]
    radius_km: f64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::try_parse()?;
    let _telemetry = telemetry::Telemetry::init(&args.log)?;
    let region = Region {
        center: geo::Point::new(args.lon, args.lat),
        radius_km: args.radius_km,
    };

    let dpc = DpcClient::new()?;
    let exporter = Arc::new(exporter::Exporter::new(dpc, region));
    let state = app::AppState { exporter };

    server::serve(app::router(state), args.bind).await
}
