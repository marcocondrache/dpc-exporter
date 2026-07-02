mod app;
mod dpc;
mod server;
mod telemetry;

use std::net::SocketAddr;

use clap::Parser;

use crate::dpc::Dpc;

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(long, env = "BIND", default_value = "0.0.0.0:8080")]
    bind: SocketAddr,

    #[arg(long, env = "RUST_LOG", default_value = "info")]
    log: String,

    /// Home latitude (WGS84).
    #[arg(long, env = "HOME_LAT")]
    lat: f64,

    /// Home longitude (WGS84).
    #[arg(long, env = "HOME_LON")]
    lon: f64,

    /// Radius around home to monitor, in km.
    #[arg(long, env = "HOME_RADIUS_KM", default_value_t = 20.0)]
    radius_km: f64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::try_parse()?;
    let _telemetry = telemetry::Telemetry::init(&args.log)?;

    let dpc = Dpc::new()?;

    server::serve(app::router(dpc), args.bind).await
}
