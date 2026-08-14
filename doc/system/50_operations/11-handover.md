# 11. Handover

## Implementation Status

**Tarcie v1.0.0** -- stable, feature-complete for v1 scope.

All modules implemented: IPC commands, JSONL queue, HTTP sink client, background flusher, data model, constraints, state management, platform paths, global hotkey.

Delivery uses a **claim**: the flush renames `queue.jsonl` into `sending/`
before it posts anything, so an event captured during a flush cannot be
archived as sent. Section 5 describes the lifecycle and section 6 the loop.
Read those two before changing anything in `flusher.rs` or `queue/jsonl.rs`.

The repository has 76 unit tests and a CI workflow that runs them on every
pull request. Section 10 lists what they cover and what they do not.

## Critical Constraints (Do Not Violate)

1. **Write-only.** No readback surfaces. No query commands. No browsing UI.
2. **No AI.** Raw strings only. No LLMs, no embeddings, no summarization.
3. **No categorization.** SMITH does grouping. Tarcie captures verbatim.
4. **Localhost-only default.** Remote sink requires explicit `TARCIE_ALLOW_REMOTE_SINK=true`.
5. **5-second capture revert.** If capture takes > 5s, revert. Never block the user.

## Known Limitations

- **Test coverage is unit-level only.** The seven priority areas in section 10
  have tests, and so do the command layer, device-ID persistence, the name
  guard, and the hotkey string. Hotkey registration, the window toggle, and the
  shutdown flush do not: all three need a running desktop session. Section 10
  lists what stays uncovered.
- **A crash between a partial delivery and its archive duplicates the
  remainder.** The undelivered events are written back before the originals are
  archived, so a crash between those two steps offers the remainder again. This
  is deliberate: the contract prefers a duplicate to a loss.
- **Duplicate delivery is possible in general.** The sink is not asked whether
  it already holds an event, so any retry after an unacknowledged success sends
  it again. Deduplication belongs downstream, on `id`.
- **Monotonic clock resets on restart.** `timestamp_mono_ms` is relative to session start. Cross-session ordering relies on `timestamp_utc` only.
- **Platform paths.** Uses the `directories` crate for queue file location. Windows IPC path edge cases have not been tested.
- **No retry persistence.** If the application is killed during a flush, partial state depends on whether the queue rotation completed. The tolerant reader handles most corruption cases.
- **No sink health check.** Tarcie does not probe the sink before flushing. It discovers sink unavailability at flush time and defers.

## Dev Quickref

```bash
# Build
cd src-tauri && cargo build

# Run in dev mode
cd src-tauri && cargo tauri dev

# Run the tests (needs dist/ — run `npm run build` first)
cd src-tauri && cargo test

# Check types
cd src-tauri && cargo check

# Lint
cd src-tauri && cargo clippy -- -W clippy::all

# Environment overrides
export TARCIE_SINK_URL="http://127.0.0.1:9090/ingest/tarcie"
export TARCIE_FLUSH_INTERVAL_SECS=60
export TARCIE_BATCH_MAX=50
```

## File Locations

| Item | Path |
|------|------|
| Rust source | `src-tauri/src/` |
| Frontend | `src/` (main.ts, styles.css, index.html) |
| Frontend bundle | `dist/` (built by `npm run build`; `cargo` needs it) |
| Cargo manifest | `src-tauri/Cargo.toml` |
| Tauri config | `src-tauri/tauri.conf.json` |
| Queue files | Platform queue dir via `directories` crate |
| Device ID | Platform data dir via `directories` crate |
