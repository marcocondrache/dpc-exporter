# hailwarden

A Prometheus exporter for local hail risk in Italy. It reads observed radar
products from the Dipartimento della Protezione Civile (DPC) and exposes them
as metrics around your home, so Grafana can dashboard and alert on them — e.g.
`hailwarden_hail_probability{scope="home"} > 0.5`.

The service does no scoring of its own: DPC already computes a Probability of
Hail field, and Grafana handles thresholds and alerting.

## ✦ Metrics

Refreshed every 5 minutes (DPC's cadence); scrapes read a cache, never fetch.

| Metric | Meaning |
| --- | --- |
| `hailwarden_hail_probability{scope}` | DPC POH, 0–1 |
| `hailwarden_reflectivity_dbz{scope}` | DPC VMI, column-max reflectivity |
| `hailwarden_vil_kgm2{scope}` | DPC VIL, vertically integrated liquid |
| `hailwarden_frame_timestamp_seconds` | Unix time of the sample (staleness) |

`scope="home"` is the value at your cell; `scope="area"` is the maximum within
the configured radius (an approaching cell).

## ✦ Run

```sh
HOME_LAT=45.46 HOME_LON=9.19 HOME_RADIUS_KM=20 cargo run
curl localhost:8080/metrics
```

Config is environment-driven (`--help` for flags).

## ✦ Data & license

Radar data © Dipartimento della Protezione Civile (Radar-DPC), distributed
under CC-BY-SA 4.0. Derived works must attribute Radar-DPC and share alike.
