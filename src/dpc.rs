use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use geotiff::GeoTiff;
use serde::Deserialize;
use std::{io::Cursor, sync::LazyLock};

const API: &str = "https://radar-api.protezionecivile.it";
const ORIGIN: &str = "https://radar.protezionecivile.it";
const NODATA: f32 = -9000.0; // DPC uses -9999; anything this low is missing.

static FROM_PROJ: LazyLock<proj4rs::Proj> = LazyLock::new(|| {
    proj4rs::Proj::from_proj_string("+proj=longlat +datum=WGS84 +no_defs").unwrap()
});

static TO_PROJ: LazyLock<proj4rs::Proj> = LazyLock::new(|| {
    proj4rs::Proj::from_proj_string(
        "+proj=tmerc +lat_0=42 +lon_0=12.5 +k=1 +x_0=0 +y_0=0 +ellps=WGS84 +units=m +no_defs",
    )
    .unwrap()
});

#[derive(Debug, Clone, Copy)]
pub struct Region {
    pub center: geo::Point<f64>,
    pub radius_km: f64,
}

pub struct DpcGrid {
    grid: Vec<Vec<f32>>,
}

impl DpcGrid {
    pub fn center(&self) -> f32 {
        let mid = self.grid.len() / 2;
        self.grid[mid][mid]
    }

    pub fn max(&self) -> f32 {
        self.grid
            .iter()
            .flatten()
            .copied()
            .filter(|v| !v.is_nan())
            .fold(f32::NAN, f32::max)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DpcProduct {
    VerticalMaximumIntensity,
    SurfaceRainfallIntensity,
    VerticallyIntegratedLiquid,
    EchoTopMap,
    ProbabilityOfHail,
}

impl DpcProduct {
    pub fn as_str(&self) -> &str {
        match self {
            DpcProduct::VerticalMaximumIntensity => "VMI",
            DpcProduct::SurfaceRainfallIntensity => "SRI",
            DpcProduct::VerticallyIntegratedLiquid => "VIL",
            DpcProduct::ProbabilityOfHail => "POH",
            DpcProduct::EchoTopMap => "ETM",
        }
    }
}

pub struct Dpc {
    client: reqwest::Client,
}

impl Dpc {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
        })
    }

    pub async fn fetch_latest(&self, product: DpcProduct) -> Result<(DateTime<Utc>, GeoTiff)> {
        let time = self.latest_time(product).await?;
        let grid = self.grid(product, time).await?;

        Ok((time, grid))
    }

    pub async fn fetch_latest_at(
        &self,
        product: DpcProduct,
        region: Region,
    ) -> Result<(DateTime<Utc>, DpcGrid)> {
        let (time, grid) = self.fetch_latest(product).await?;
        let mut center = (
            region.center.x().to_radians(),
            region.center.y().to_radians(),
            0.0,
        );

        proj4rs::transform::transform(&FROM_PROJ, &TO_PROJ, &mut center)?;

        let center = geo::Coord {
            x: center.0,
            y: center.1,
        };

        // All DPC products share one grid geometry; derive the cell size from
        // the model extent (1 km for DPC).
        let ext = grid.model_extent();
        let step = ext.width() / grid.raster_width as f64;

        grid.get_value_at::<f32>(&center, 0)
            .context("home is outside DPC radar coverage")?;

        Ok((
            time,
            Self::crop(&grid, step, region.radius_km.ceil() as usize, center),
        ))
    }

    async fn latest_time(&self, product: DpcProduct) -> Result<DateTime<Utc>> {
        #[derive(Deserialize)]
        struct LatestResponse {
            #[serde(rename = "lastProducts")]
            last_products: Vec<Latest>,
        }

        #[derive(Deserialize)]
        struct Latest {
            time: i64,
        }

        let resp: LatestResponse = self
            .client
            .get(format!("{API}/findLastProductByType"))
            .query(&[("type", product.as_str())])
            .header("origin", ORIGIN)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        resp.last_products
            .first()
            .map(|l| l.time)
            .and_then(|t| Utc.timestamp_millis_opt(t).single())
            .context("No product available")
    }

    async fn grid(&self, product: DpcProduct, time: DateTime<Utc>) -> Result<GeoTiff> {
        #[derive(Deserialize)]
        struct Download {
            url: String,
        }

        let download: Download = self
            .client
            .post(format!("{API}/downloadProduct"))
            .header("origin", ORIGIN)
            .json(&serde_json::json!({ "productType": product.as_str(), "productDate": time.timestamp_millis() }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let bytes = self
            .client
            .get(&download.url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        tokio::task::spawn_blocking(|| GeoTiff::read(Cursor::new(bytes)))
            .await?
            .map_err(Into::into)
    }

    fn crop(g: &GeoTiff, step: f64, radius: usize, center: geo::Coord<f64>) -> DpcGrid {
        let value_at = |g: &GeoTiff, coord: geo::Coord<f64>| match g.get_value_at::<f32>(&coord, 0)
        {
            Some(v) if v > NODATA => v,
            _ => f32::NAN,
        };

        let cell_coord =
            |center: geo::Coord<f64>, step: f64, half_cells: usize, r: usize, c: usize| {
                geo::Coord {
                    x: center.x + (c as f64 - half_cells as f64) * step,
                    y: center.y + (half_cells as f64 - r as f64) * step,
                }
            };

        let grid = (0..=2 * radius)
            .map(|r| {
                (0..=2 * radius)
                    .map(|c| value_at(g, cell_coord(center, step, radius, r, c)))
                    .collect()
            })
            .collect();

        DpcGrid { grid }
    }
}
