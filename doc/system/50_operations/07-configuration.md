# 7. Configuration

All configuration is via environment variables. There is no config file. Defaults are safe for local development.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `TARCIE_SINK_URL` | `http://127.0.0.1:8080/ingest/tarcie` | HTTP endpoint for event ingestion |
| `TARCIE_ALLOW_REMOTE_SINK` | `false` | If `false`, sink URL must be localhost/127.0.0.1. Safety constraint |
| `TARCIE_SINK_AUTH` | *(none)* | Optional value for the `Authorization` header on sink requests |
| `TARCIE_FLUSH_INTERVAL_SECS` | `300` | Seconds between background flush cycles |
| `TARCIE_BATCH_MAX` | `200` | Maximum events per HTTP POST batch |
| `TARCIE_QUEUE_MAX_EVENTS` | `10000` | Queue cap -- triggers rotation when reached |

## Localhost-Only Default

By default, `TARCIE_ALLOW_REMOTE_SINK` is `false`. This means the sink URL must resolve to `127.0.0.1` or `localhost`. Any attempt to configure a remote sink URL without explicitly setting `TARCIE_ALLOW_REMOTE_SINK=true` will be rejected at startup.

This is a safety constraint: Tarcie captures raw, unfiltered user text. Sending it to a remote endpoint without explicit opt-in would be a data leak.

## Configuration Source

All config is read in `sink/config.rs` and assembled into a `SinkConfig` struct at application startup. The config is immutable for the lifetime of the process.
