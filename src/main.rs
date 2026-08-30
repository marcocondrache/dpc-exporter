mod app;
mod dpc;
mod exporter;
mod server;
mod telemetry;
mod types;

use std::{net::SocketAddr, sync::Arc};

use clap::Parser;

use crate::{
    dpc::{DpcClient, Region},
    types::{Latitude, Longitude, RadiusKm},
};

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(long, env = "BIND", default_value = "0.0.0.0:8080")]
    bind: SocketAddr,

    #[arg(long, env = "RUST_LOG", default_value = "info")]
    log: String,

    /// latitude (WGS84).
    #[arg(long, env = "LAT")]
    lat: Latitude,

    /// longitude (WGS84).
    #[arg(long, env = "LON")]
    lon: Longitude,

    /// Radius around to monitor, in km.
    #[arg(long, env = "RADIUS_KM", default_value = "20")]
    radius_km: RadiusKm,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::try_parse()?;
    let _telemetry = telemetry::Telemetry::init(&args.log)?;
    let region = Region {
        center: geo::Point::new(*args.lon, *args.lat),
        radius_km: *args.radius_km,
    };

    let dpc = DpcClient::new()?;
    let exporter = Arc::new(exporter::Exporter::new(dpc, region));
    let state = app::AppState { exporter };

    server::serve(app::router(state), args.bind).await
}
