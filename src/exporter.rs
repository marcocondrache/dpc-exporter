use std::sync::{Mutex, MutexGuard};

use chrono::{DateTime, Utc};

use crate::dpc::{DpcClient, DpcGrid, DpcProduct, Region};

/// DPC publishes a new sample every 5 minutes; younger cache needs no fetch.
const SAMPLE_PERIOD_SECS: i64 = 300;

pub struct ExporterFrame {
    pub time: DateTime<Utc>,
    pub poh: DpcGrid,
    pub vmi: DpcGrid,
    pub vil: DpcGrid,
    pub sri: DpcGrid,
    pub etm: DpcGrid,
}

pub struct Exporter {
    dpc: DpcClient,
    region: Region,
    frame: Mutex<Option<ExporterFrame>>,
}

impl Exporter {
    pub fn new(dpc: DpcClient, region: Region) -> Self {
        Self {
            dpc,
            region,
            frame: Mutex::new(None),
        }
    }

    pub fn frame(&self) -> MutexGuard<'_, Option<ExporterFrame>> {
        self.frame.lock().unwrap()
    }

    pub async fn refresh(&self) {
        {
            let frame = self.frame.lock().unwrap();
            if let Some(f) = frame.as_ref()
                && (Utc::now() - f.time).num_seconds() < SAMPLE_PERIOD_SECS
            {
                return;
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

    async fn fetch(&self) -> anyhow::Result<ExporterFrame> {
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

        Ok(ExporterFrame {
            time: poh.0,
            poh: poh.1,
            vmi: vmi.1,
            vil: vil.1,
            sri: sri.1,
            etm: etm.1,
        })
    }
}
