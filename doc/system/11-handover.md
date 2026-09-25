# 11. Handover

## Implementation Status

**Tarcie v1.0.0** -- stable, feature-complete for v1 scope.

All modules implemented: IPC commands, JSONL queue, HTTP sink client, background flusher, data model, constraints, state management, platform paths, global hotkey.

Delivery uses a **claim**: the flush renames `queue.jsonl` into `sending/`
before it posts anything, so an event captured during a flush cannot be
archived as sent. Section 5 describes the lifecycle and section 6 the loop.
Read those two before changing anything in `flusher.rs` or `queue/jsonl.rs`.

The repository has 126 Rust unit tests and 29 frontend unit tests, and a CI
workflow that runs both on every pull request. Section 10 lists what they cover
and what they do not.

## Critical Constraints (Do Not Violate)

1. **Write-only.** No readback surfaces. No query commands. No browsing UI.
2. **No AI.** Raw strings only. No LLMs, no embeddings, no summarization.
3. **No categorization.** SMITH does grouping. Tarcie captures verbatim.
4. **Localhost-only default.** Remote sink requires explicit `TARCIE_ALLOW_REMOTE_SINK=true`.
5. **5-second capture revert.** If capture takes > 5s, revert. Never block the user.

## Known Limitations

- **Test coverage is unit-level only.** The seven priority areas in section 10
  have tests, and so do the command layer, device-ID persistence, the name
  guard, the hotkey string, and the capture revert. Hotkey registration, the
  window toggle, the shutdown flush, and the DOM wiring do not: the first three
  need a running desktop session, and the fourth needs a DOM in the test run.
  Section 10 lists what stays uncovered.
- **The queue grows on disk while the sink is unreachable.** Cap rotation
  bounds the size of one file and nothing bounds the number of files, so a sink
  that stays down costs disk without limit. The contract prefers that to
  discarding a capture, and undelivered events cannot age out the way delivered
  ones do. Memory is bounded: a claim takes at most `CLAIM_MAX_EVENTS`, so the
  backlog is no longer parsed in full on every cycle. What a capture pays is
  bounded too: the cap is checked against a count held in memory rather than by
  reading the queue file back, so an append costs the same whatever the backlog.
- **Captures are stored in plain text.** The queue and the archive hold notes
  verbatim. Both are closed to other accounts by their directory modes, and the
  archive is bounded at 90 days and 256 MiB, so it no longer grows for the life
  of the installation. Neither is encrypted: anything running as the same user
  reads them as easily as tarcie does, and encryption at rest stays an open
  decision. The log holds no capture content by invariant and the device ID is
  a random UUID, so the queue and the archive are the two surfaces that carry
  anything worth encrypting.
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

# Run the frontend tests and typecheck
npm test
npm run check

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
| Frontend | `src/` (main.ts, overlay.ts, capture.ts, styles.css, index.html) |
| Frontend bundle | `dist/` (built by `npm run build`; `cargo` needs it) |
| Cargo manifest | `src-tauri/Cargo.toml` |
| Tauri config | `src-tauri/tauri.conf.json` |
| Queue files | Platform queue dir via `directories` crate |
| Device ID | Platform data dir via `directories` crate |
| Log | `<data dir>/logs/tarcie.log`, with one previous file beside it |
