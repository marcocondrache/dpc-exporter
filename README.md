# dpc-exporter

A Prometheus exporter for radar products from the Dipartimento della
Protezione Civile (DPC). It reads observed radar products and exposes them as
metrics around a configured point, so consumers can dashboard and alert on
them, e.g. `dpc_hail_probability{scope="center"} > 0.5`.

The service does no scoring of its own: DPC already computes a Probability of
Hail field, and consumers handles thresholds and alerting.

## ✦ Metrics

Refreshed every 5 minutes (DPC's cadence); scrapes read a cache, never fetch.

| Metric | Meaning |
| --- | --- |
| `dpc_hail_probability{scope}` | DPC POH, 0–1 |
| `dpc_reflectivity_dbz{scope}` | DPC VMI, column-max reflectivity |
| `dpc_vil_kgm2{scope}` | DPC VIL, vertically integrated liquid |
| `dpc_rain_intensity_mmh{scope}` | DPC SRI, surface rainfall intensity |
| `dpc_echo_top_meters{scope}` | DPC ETM, echo top height |
| `dpc_frame_timestamp_seconds` | Unix time of the sample (staleness) |

`scope="center"` is the value at your cell; `scope="area"` is the maximum
within the configured radius (an approaching cell).

## ✦ Run

```sh
cargo run -- --lat 45.46 --lon 9.19 --radius-km 20
curl localhost:8080/metrics
```

Config is flag-driven, with env-var fallbacks (`--help` for all flags).

## ✦ Data & license

Radar data © Dipartimento della Protezione Civile (Radar-DPC), distributed
under CC-BY-SA 4.0. Derived works must attribute Radar-DPC and share alike.
