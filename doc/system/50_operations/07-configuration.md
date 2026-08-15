# 7. Configuration

All configuration is via environment variables. There is no config file. Defaults are safe for local development.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `TARCIE_SINK_URL` | `http://127.0.0.1:8080/ingest/tarcie` | HTTP endpoint for event ingestion |
| `TARCIE_ALLOW_REMOTE_SINK` | `false` | If `false`, sink URL must be localhost/127.0.0.1. Safety constraint |
| `TARCIE_SINK_AUTH` | *(none)* | Optional value for the `Authorization` header on sink requests |
| `TARCIE_FLUSH_INTERVAL_SECS` | `300` | Seconds between background flush cycles |
| `TARCIE_FLUSH_AT` | *(none)* | Local `HH:MM` for a daily delivery. When set, the interval becomes how often the schedule is checked |
| `TARCIE_BATCH_MAX` | `200` | Maximum events per HTTP POST batch |
| `TARCIE_QUEUE_MAX_EVENTS` | `10000` | Queue cap -- triggers rotation when reached |

## Floors

Three of the numeric settings are floored at the smallest value that still
works, because a value below the floor disables the thing it configures:

| Variable | Floor | Below it |
|----------|-------|----------|
| `TARCIE_FLUSH_INTERVAL_SECS` | `1` | `tokio::time::interval` panics on a zero period, in the spawned flush task where nothing reports it |
| `TARCIE_BATCH_MAX` | `1` | A zero batch never drains the queue |
| `TARCIE_QUEUE_MAX_EVENTS` | `100` | A cap this low rotates on almost every append |

An unparsable value is not an error. It falls back to the default, so a typo
costs the override and not the launch.

## Two Delivery Modes

Leaving `TARCIE_FLUSH_AT` unset keeps the original behaviour: every tick of
`TARCIE_FLUSH_INTERVAL_SECS` delivers.

Setting it to a local `HH:MM` makes delivery daily. The interval then stops
meaning "how often to deliver" and becomes "how often to ask whether today's
delivery is owed", so a target of `02:00` with the default 300-second interval
delivers within five minutes of two in the morning.

A value that is not a time is not an error. It falls back to the interval and
says so in the log, because falling back delivers more often rather than less,
and a typo must not be the reason a night goes missing.

Section 6 describes what a scheduled delivery does when it runs, and why a
missed night is recoverable.

## Localhost-Only Default

By default, `TARCIE_ALLOW_REMOTE_SINK` is `false`. This means the sink URL must resolve to `127.0.0.1` or `localhost`. Any attempt to configure a remote sink URL without explicitly setting `TARCIE_ALLOW_REMOTE_SINK=true` will be rejected at startup.

This is a safety constraint: Tarcie captures raw, unfiltered user text. Sending it to a remote endpoint without explicit opt-in would be a data leak.

## Configuration Source

All config is read in `sink/config.rs` and assembled into a `SinkConfig` struct at application startup. The config is immutable for the lifetime of the process.
