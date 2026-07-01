use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use geo::{Point, Rect};
use serde::{Deserialize, Serialize};

/// An area around a center point that a source should cover.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Region {
    pub center: Point<f64>,
    pub radius_km: f64,
}

/// A single radar sweep, reflectivity normalized to dBZ on a regular grid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarFrame {
    pub time: DateTime<Utc>,
    /// Geographic bounds of the covered area (min = SW corner, max = NE corner).
    pub bounds: Rect<f64>,
    pub resolution_m: f32,
    /// Reflectivity grid; rows north->south, cols west->east.
    pub dbz: Vec<Vec<f32>>,
}

/// The convective environment at a location for one hour.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EnvironmentSnapshot {
    pub time: DateTime<Utc>,
    /// Convective energy; updraft potential (J/kg).
    pub cape_j_kg: f32,
    /// Convective inhibition; cap strength (J/kg).
    pub cin_j_kg: f32,
    /// Height of the melt line (m); low favors hail reaching the ground.
    pub freezing_level_m: f32,
    /// 500 hPa flow: expected cell motion speed (km/h).
    pub steering_wind_kmh: f32,
    /// 500 hPa flow: expected cell motion bearing.
    pub steering_dir: f32,
    pub wind_gust_kmh: f32,
    pub precip_mm: f32,
}

/// Lightning activity around a region over recent windows.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LightningSummary {
    pub time: DateTime<Utc>,
    pub strikes_5m_5km: u32,
    pub strikes_15m_30km: u32,
    /// Rate-of-change / lightning-jump indicator, normalized [0, 1].
    pub jump_score: f32,
}

/// A radar reflectivity source
#[async_trait]
pub trait RadarSource: Send + Sync {
    fn name(&self) -> &'static str;
    /// Recent sweeps covering `region`, oldest first.
    async fn frames(&self, region: &Region) -> Result<Vec<RadarFrame>>;
}

/// A convective-environment source
#[async_trait]
pub trait EnvironmentSource: Send + Sync {
    fn name(&self) -> &'static str;
    /// Environment for the current/next hour at `location`.
    async fn snapshot(&self, location: &Point<f64>) -> Result<EnvironmentSnapshot>;
}

/// A lightning source
#[async_trait]
pub trait LightningSource: Send + Sync {
    fn name(&self) -> &'static str;
    /// Lightning summary for `region`.
    async fn summary(&self, region: &Region) -> Result<LightningSummary>;
}
