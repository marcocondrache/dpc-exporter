use nutype::nutype;

/// Latitude in WGS84, within [-90, 90].
#[nutype(
    validate(finite, greater_or_equal = -90.0, less_or_equal = 90.0),
    derive(Debug, Clone, Copy, FromStr, Deref)
)]
pub struct Latitude(f64);

/// Longitude in WGS84, within [-180, 180].
#[nutype(
    validate(finite, greater_or_equal = -180.0, less_or_equal = 180.0),
    derive(Debug, Clone, Copy, FromStr, Deref)
)]
pub struct Longitude(f64);

/// Radius to monitor around the center, in km: finite, within (0, 200].
#[nutype(
    validate(finite, greater = 0.0, less_or_equal = 200.0),
    derive(Debug, Clone, Copy, FromStr, Deref)
)]
pub struct RadiusKm(f64);
