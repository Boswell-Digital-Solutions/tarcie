# tarcie - Compiled System Reference

**Designation:** TAR
**Document role:** Canonical compiled technical reference for the Tarcie local capture tool
**Source:** `doc/system/`
**Build command:** `bash doc/system/BUILD.sh`
**Document version:** 2.0 (2026-06-22) - canonical compliance migration
**Protocol:** BDS Documentation Protocol v2.0; BDS Repo Documentation System Canonical Compliance Standard

> **Generated artifact warning:** `doc/TARSYSTEM.md` is assembled output. Edit
> the source modules under `doc/system/` and rebuild. Hand edits to the
> compiled artifact are overwritten by the next build.

Assembly contract:

- Command: `bash doc/system/BUILD.sh`
- Validation: `bash doc/system/validate_snapshots.sh` runs during assembly
- Primary output: `doc/TARSYSTEM.md`

This `doc/system/` tree is the canonical source of truth for tarcie. It
uses explicit **truth classes**: canonical facts define the repo role, authority
boundaries, runtime behavior, service contracts, and verification doctrine;
snapshot facts are dated, audit-derived counts and current implementation
inventory that may drift between audits.

| Part | File | Contents |
| --- | --- | --- |
| §1 | `00_overview/01-overview.md` | 1. Overview |
| §2 | `00_overview/02-architecture.md` | 2. Architecture |
| §3 | `10_service-contract/03-command-reference.md` | 3. Command Reference |
| §4 | `10_service-contract/10-product-surface.md` | Product Surface |
| §5 | `20_runtime/04-data-model.md` | 4. Data Model |
| §6 | `20_runtime/05-queue-system.md` | 5. Queue System |
| §7 | `20_runtime/06-flush-pipeline.md` | 6. Flush Pipeline |
| §8 | `20_runtime/09-error-handling.md` | 9. Error Handling |
| §9 | `20_runtime/20-runtime.md` | Runtime |
| §10 | `30_dependencies/40-integrations.md` | Integrations |
| §11 | `50_operations/07-configuration.md` | 7. Configuration |
| §12 | `50_operations/08-constraints.md` | 8. Constraints |
| §13 | `50_operations/10-testing.md` | 10. Testing |
| §14 | `50_operations/11-handover.md` | 11. Handover |
| §15 | `50_operations/50-operations.md` | Operations |
| §16 | `99_appendices/30-data.md` | 4. Data Model |
| §17 | `99_appendices/90-appendices.md` | Appendices |
| §18 | `99_appendices/91-bootstrap-overview.md` | 1. Overview |
| §19 | `99_appendices/92-bootstrap-architecture.md` | 2. Architecture |

## Quick Assembly

```bash
bash doc/system/BUILD.sh
```

---

# 1. Overview

Tarcie is a friction-free capture tool for notes and markers. A global hotkey (`Ctrl+Alt+T`) pops a 480x140px overlay window. The user types a note or drops a timestamp marker. The event is appended to a JSONL file queue with fsync durability, and a background flusher periodically batches events to an HTTP sink endpoint.

## Design Philosophy

Tarcie is strictly **write-only** in v1. There is no readback surface, no categorization logic, and no AI processing. Raw strings go in, raw strings go out. SMITH handles grouping and analysis downstream.

## At a Glance

| Metric | Value |
|--------|-------|
| Rust LOC | 636 |
| IPC Commands | 3 |
| Frontend | Vanilla TypeScript (main.ts + styles.css + index.html) |
| Framework | Tauri 2.0 (no SvelteKit, no UI framework) |
| Edition | Rust 2024 |
| Version | 1.0.0 |

## What Tarcie Does

1. Listens for `Ctrl+Alt+T` global hotkey
2. Shows/hides a minimal overlay window
3. Accepts text notes (with optional `#tag`) or timestamp markers
4. Appends events to a local JSONL queue (fsync-durable)
5. Flushes queued events to an HTTP sink in batches on a timer
6. Attempts a graceful flush on shutdown (5-second timeout)

## What Tarcie Does Not Do

- No readback or browsing of captured events
- No categorization, tagging intelligence, or grouping
- No AI/LLM processing of any kind
- No complex UI beyond the capture overlay

---

# 2. Architecture

## System Diagram

```
  User
   │
   │  Ctrl+Alt+T
   ▼
┌──────────────────────┐
│  Overlay Window       │  480x140px, vanilla TS
│  (main.ts + index.html)│
└──────────┬───────────┘
           │ Tauri IPC
           ▼
┌──────────────────────┐
│  IPC Commands         │
│  ├─ capture_note()    │
│  ├─ capture_marker()  │
│  └─ flush_now()       │
└──────────┬───────────┘
           │ TarcieEvent
           ▼
┌──────────────────────┐
│  JSONL Queue          │  Mutex-protected, fsync append
│  queue.jsonl          │
└──────────┬───────────┘
           │ Background timer
           ▼
┌──────────────────────┐
│  Flusher              │  Batch POST, exp. backoff
│  ├─ Read queue        │
│  ├─ Chunk into batches│
│  └─ POST to sink      │
└──────────┬───────────┘
           │ HTTP POST
           ▼
┌──────────────────────┐
│  HTTP Sink            │  default: 127.0.0.1:8080
│  /ingest/tarcie       │  localhost-only by default
└──────────────────────┘
```

## Module Map

| Module | Files | Purpose |
|--------|-------|---------|
| `ipc/` | `commands.rs`, `mod.rs` | 3 Tauri IPC commands |
| `queue/` | `jsonl.rs`, `mod.rs` | JSONL file queue (append, read, rotate) |
| `sink/` | `client.rs`, `config.rs`, `mod.rs` | HTTP sink client + env-based config |
| `flusher.rs` | -- | Background flush loop with retry and batch posting |
| `model.rs` | -- | `TarcieEvent` struct + `EventType` enum |
| `constraints.rs` | -- | All v1 hard limits and constants |
| `state.rs` | -- | `AppState` (config, queue, flusher, device_id, mono_start) |
| `util/` | paths module | Platform directory paths via `directories` crate |

## Data Flow

1. User presses `Ctrl+Alt+T` -- overlay toggles visibility
2. User types text and submits -- frontend calls `capture_note` or `capture_marker` via Tauri IPC
3. Command builds a `TarcieEvent` with UUID, device ID, UTC + monotonic timestamps
4. Event is serialized to JSON and appended to `queue.jsonl` with fsync
5. Background flusher wakes on interval (default 300s) or manual `flush_now`
6. Flusher reads all events, batches them (max 200 per batch), POSTs to sink
7. On success: queue file rotated to `queue.sent.TIMESTAMP.jsonl`
8. On failure after retries: events remain in queue for next attempt

## Dependencies

| Crate | Purpose |
|-------|---------|
| `tauri` 2.x | Desktop application framework |
| `tauri-plugin-global-shortcut` | `Ctrl+Alt+T` hotkey registration |
| `serde` + `serde_json` | Serialization |
| `uuid` | Event and device ID generation |
| `chrono` | UTC timestamps |
| `directories` | Platform-appropriate file paths |
| `tokio` | Async runtime |
| `reqwest` (rustls-tls) | HTTP client for sink |
| `anyhow` | Error handling |
| `regex` | Tag extraction from note content |
| `url` | Sink URL validation |

---

# 3. Command Reference

Tarcie exposes exactly 3 IPC commands via Tauri. All commands are in `ipc/commands.rs`.

## capture_note

```rust
#[tauri::command]
pub async fn capture_note(
    content: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String>
```

**Purpose:** Capture a text note.

**Behavior:**
1. Clamp `content` to `MAX_CONTENT_BYTES` (10 KB)
2. Extract first `#tag` from content (if present) as `app_context`, clamped to `MAX_TAG_CHARS` (32)
3. Build a `TarcieEvent` with:
   - Fresh UUID
   - Device ID from state
   - UTC timestamp + monotonic offset
   - `EventType::Note`
4. Append event to JSONL queue (fsync-durable)
5. Return `"ok"` on success

**Errors:** Returns stringified error on queue write failure.

---

## capture_marker

```rust
#[tauri::command]
pub async fn capture_marker(
    reason: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<String, String>
```

**Purpose:** Drop a timestamp marker event.

**Behavior:**
1. Build a `TarcieEvent` with:
   - Fresh UUID
   - Device ID from state
   - UTC timestamp + monotonic offset
   - `EventType::Marker { reason }` (reason is optional, clamped if provided)
2. Append event to JSONL queue (fsync-durable)
3. Return `"ok"` on success

**Errors:** Returns stringified error on queue write failure.

---

## flush_now

```rust
#[tauri::command]
pub async fn flush_now(
    state: tauri::State<'_, AppState>,
) -> Result<String, String>
```

**Purpose:** Trigger an immediate flush of the queue to the sink.

**Behavior:**
1. Attempt to acquire flush lock
2. Read all events from queue
3. Batch and POST to sink endpoint
4. Return one of:
   - `"empty"` -- queue had no events
   - `"ok:N"` -- successfully flushed N events
   - `"deferred:reason"` -- flush could not complete (events remain in queue)

**Errors:** Returns stringified error on unexpected failures.

---

# Product Surface

**Document version:** 1.0 (bootstrap scaffold)

User-facing product surface: routes, flows, and entry points.

> This chapter is a registry-generated bootstrap scaffold for a
> `application` class documentation system. Replace this placeholder with
> real authored content. Registry will not invent repo truth that is not
> already present in the repo.

---

# 4. Data Model

## TarcieEvent

The core data structure for all captured events.

```rust
pub struct TarcieEvent {
    pub id: Uuid,
    pub device_id: Uuid,
    pub timestamp_utc: DateTime<Utc>,
    pub timestamp_mono_ms: u64,
    pub event_type: EventType,
    pub content: String,
    pub app_context: String,
    pub source_version: String,
}
```

### Field Descriptions

| Field | Type | Description |
|-------|------|-------------|
| `id` | `Uuid` | Unique event identifier, generated per capture |
| `device_id` | `Uuid` | Persistent device identifier, created on first launch and stored to disk |
| `timestamp_utc` | `DateTime<Utc>` | Wall-clock time for cross-session ordering |
| `timestamp_mono_ms` | `u64` | Monotonic clock offset in milliseconds from session start. Resets on restart. Used for relative timing within a session |
| `event_type` | `EventType` | Discriminator: `Note` or `Marker` |
| `content` | `String` | Captured text content. Max 10 KB. Empty string for bare markers |
| `app_context` | `String` | Extracted `#tag` from note content, or empty. Max 64 chars |
| `source_version` | `String` | Always `"tarcie-v1.0.0"` in v1 |

## EventType

```rust
pub enum EventType {
    Note,
    Marker { reason: Option<String> },
}
```

| Variant | Description |
|---------|-------------|
| `Note` | A text note. Content holds the user's text. First `#tag` extracted to `app_context` |
| `Marker { reason }` | A timestamp marker. Optional `reason` string describes what is being marked |

## Serialization Format

Events are serialized as JSON, one per line (JSONL). Example:

```json
{"id":"a1b2c3d4-...","device_id":"e5f6a7b8-...","timestamp_utc":"2026-02-25T14:30:00Z","timestamp_mono_ms":42000,"event_type":"Note","content":"Remember to check the flush interval #config","app_context":"config","source_version":"tarcie-v1.0.0"}
```

Marker example:

```json
{"id":"f9e8d7c6-...","device_id":"e5f6a7b8-...","timestamp_utc":"2026-02-25T14:31:00Z","timestamp_mono_ms":102000,"event_type":{"Marker":{"reason":"deploy started"}},"content":"","app_context":"","source_version":"tarcie-v1.0.0"}
```

## Sink Payload

When flushed, events are batched into a JSON payload:

```json
{
  "source": "tarcie",
  "events": [ ... ]
}
```

Each batch contains up to `DEFAULT_BATCH_MAX` (200) events.

---

# 5. Queue System

The queue is the durable buffer between capture and flush. Implemented in `queue/jsonl.rs`.

## File Format

- **Format:** JSONL (one JSON object per line)
- **File:** `queue.jsonl` in the platform-appropriate queue directory (via `directories` crate)
- **Encoding:** UTF-8

## Append

1. Serialize `TarcieEvent` to a JSON string
2. Sanity-parse the string back (catch serialization bugs early)
3. Append the line to `queue.jsonl`
4. `fsync` the file (durability guarantee)

All appends are protected by a `Mutex` to prevent interleaved writes from concurrent IPC calls.

## Read (Tolerant)

The queue reader is tolerant of malformed lines:

- Each line is attempted as JSON deserialization
- Malformed lines are skipped with a warning (not fatal)
- Processing continues to the next line

This ensures a single corrupted event never blocks the entire queue.

## Rotation

### Cap Rotation

When the queue reaches `DEFAULT_QUEUE_MAX_EVENTS` (10,000 events), the current `queue.jsonl` is renamed to:

```
queue.cap.{TIMESTAMP}.jsonl
```

A fresh `queue.jsonl` is created for new events. This prevents unbounded file growth if the sink is unreachable for an extended period.

### Success Rotation

After a successful flush, the queue file is renamed to:

```
queue.sent.{TIMESTAMP}.jsonl
```

This preserves a local record of sent events while clearing the active queue.

## Capacity

| Parameter | Default |
|-----------|---------|
| Max events before cap rotation | 10,000 |
| Max content per event | 10 KB |
| Max batch size per flush | 200 events |

---

# 6. Flush Pipeline

The flusher is a background task that periodically drains the JSONL queue and posts events to the HTTP sink. Implemented in `flusher.rs`.

## Flush Loop

1. Sleep for `TARCIE_FLUSH_INTERVAL_SECS` (default: 300 seconds)
2. Acquire the flush Mutex
3. Read all events from `queue.jsonl` (tolerant read)
4. If queue is empty, release lock and return to step 1
5. Chunk events into batches of `DEFAULT_BATCH_MAX` (200)
6. POST each batch to the sink endpoint
7. On success: rotate queue file to `queue.sent.TIMESTAMP.jsonl`
8. On failure after retries: return `Deferred` (events stay in queue for next cycle)
9. Release lock, return to step 1

## Batch Payload

Each HTTP POST sends:

```json
{
  "source": "tarcie",
  "events": [ <up to 200 TarcieEvent objects> ]
}
```

## Retry Strategy

- **Max retries:** 3 attempts per batch
- **Backoff:** Exponential -- `2^retry` seconds (2s, 4s, 8s)
- **On exhaustion:** Flush returns `Deferred` with reason. Events remain in the queue file untouched

## FlushResult

The flusher returns one of three outcomes:

| Result | Meaning |
|--------|---------|
| `Empty` | Queue had no events to flush |
| `Success(n)` | Successfully flushed `n` events, queue rotated |
| `Deferred(reason)` | Flush failed after retries, events remain in queue |

## Manual Flush

The `flush_now` IPC command triggers an immediate flush cycle outside the timer. It follows the same logic as the background loop but returns the result directly to the caller.

## Graceful Shutdown

On window close, Tarcie attempts a final flush with a **5-second timeout**. If the flush does not complete within 5 seconds, the application exits and events remain safely in the queue file for the next launch.

---

# 9. Error Handling

Tarcie uses a simple, pragmatic error handling strategy appropriate for a small capture tool.

## Internal Errors

All internal error handling uses the `anyhow` crate. Functions return `anyhow::Result<T>` for flexible error propagation with context.

## IPC Boundary

Tauri IPC commands must return `Result<String, String>`. All internal errors are mapped at the IPC boundary:

```rust
.map_err(|e| e.to_string())
```

This converts any `anyhow::Error` into a human-readable string for the frontend. There is no structured error type crossing IPC -- Tarcie's frontend does not inspect error details.

## FlushResult

The flusher uses a dedicated result enum rather than `Result<T, E>`:

| Variant | Meaning |
|---------|---------|
| `Empty` | Queue had nothing to flush (not an error) |
| `Success(usize)` | Flushed N events successfully |
| `Deferred(String)` | Could not flush; reason string explains why. Events remain in queue |

`Deferred` is not a panic condition. It means the sink is temporarily unreachable and events are safe in the queue file. The next flush cycle will retry.

## Queue Read Tolerance

The JSONL reader skips malformed lines rather than failing the entire read. This means:

- A single corrupted line does not block queue processing
- Malformed lines are logged as warnings
- All valid events in the file are still processed

This is intentional: data durability of valid events is prioritized over strict consistency.

## Philosophy

Tarcie does not fail loudly to the user. Capture must feel instant and invisible. Errors are logged internally but the overlay never shows error dialogs or failure states. If a capture fails, the 5-second revert constraint ensures the user is not blocked.

---

# Runtime

**Document version:** 1.0 (bootstrap scaffold)

Runtime topology, process boundaries, and managed state.

> This chapter is a registry-generated bootstrap scaffold for a
> `application` class documentation system. Replace this placeholder with
> real authored content. Registry will not invent repo truth that is not
> already present in the repo.

---

# 4. Data Model

## TarcieEvent

The core data structure for all captured events.

```rust
pub struct TarcieEvent {
    pub id: Uuid,
    pub device_id: Uuid,
    pub timestamp_utc: DateTime<Utc>,
    pub timestamp_mono_ms: u64,
    pub event_type: EventType,
    pub content: String,
    pub app_context: String,
    pub source_version: String,
}
```

### Field Descriptions

| Field | Type | Description |
|-------|------|-------------|
| `id` | `Uuid` | Unique event identifier, generated per capture |
| `device_id` | `Uuid` | Persistent device identifier, created on first launch and stored to disk |
| `timestamp_utc` | `DateTime<Utc>` | Wall-clock time for cross-session ordering |
| `timestamp_mono_ms` | `u64` | Monotonic clock offset in milliseconds from session start. Resets on restart. Used for relative timing within a session |
| `event_type` | `EventType` | Discriminator: `Note` or `Marker` |
| `content` | `String` | Captured text content. Max 10 KB. Empty string for bare markers |
| `app_context` | `String` | Extracted `#tag` from note content, or empty. Max 64 chars |
| `source_version` | `String` | Always `"tarcie-v1.0.0"` in v1 |

## EventType

```rust
pub enum EventType {
    Note,
    Marker { reason: Option<String> },
}
```

| Variant | Description |
|---------|-------------|
| `Note` | A text note. Content holds the user's text. First `#tag` extracted to `app_context` |
| `Marker { reason }` | A timestamp marker. Optional `reason` string describes what is being marked |

## Serialization Format

Events are serialized as JSON, one per line (JSONL). Example:

```json
{"id":"a1b2c3d4-...","device_id":"e5f6a7b8-...","timestamp_utc":"2026-02-25T14:30:00Z","timestamp_mono_ms":42000,"event_type":"Note","content":"Remember to check the flush interval #config","app_context":"config","source_version":"tarcie-v1.0.0"}
```

Marker example:

```json
{"id":"f9e8d7c6-...","device_id":"e5f6a7b8-...","timestamp_utc":"2026-02-25T14:31:00Z","timestamp_mono_ms":102000,"event_type":{"Marker":{"reason":"deploy started"}},"content":"","app_context":"","source_version":"tarcie-v1.0.0"}
```

## Sink Payload

When flushed, events are batched into a JSON payload:

```json
{
  "source": "tarcie",
  "events": [ ... ]
}
```

Each batch contains up to `DEFAULT_BATCH_MAX` (200) events.

---

---

# Integrations

**Document version:** 1.0 (bootstrap scaffold)

External integrations, upstream services, and wire contracts.

> This chapter is a registry-generated bootstrap scaffold for a
> `application` class documentation system. Replace this placeholder with
> real authored content. Registry will not invent repo truth that is not
> already present in the repo.

---

# Governance

**Truth class:** canonical doctrine

This documentation system governs Tarcie's repo-local implementation truth. It
does not define ecosystem-level doctrine, DataForge truth ownership, or SMITH
downstream analysis behavior beyond the contract surfaces Tarcie consumes or
hands off to.

## Authority Boundary

- `doc/system/` is the canonical authored source tree for Tarcie system truth.
- `doc/TARSYSTEM.md` is generated output and must not be edited by hand.
- Supporting docs, plans, and archives outside `doc/system/` are subordinate to
  the compiled system reference when they describe current behavior.
- Runtime behavior and verification evidence override stale prose; when they
  disagree, update the source chapter and rebuild the compiled artifact.

## Change Control

Changes that alter capture behavior, queue durability, flush semantics,
configuration, sink contracts, or safety constraints must update the relevant
`doc/system/` chapter in the same change as the implementation.

Documentation-only changes must still rebuild `doc/TARSYSTEM.md` with:

```bash
bash doc/system/BUILD.sh
```

---

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

---

# 8. Constraints

All v1 constraints are hardcoded in `constraints.rs`. They are non-negotiable.

## The Five Rules

### 1. Capture Latency: 5-Second Revert

If any capture operation (note or marker) takes longer than 5 seconds, the operation must revert. The user must never be blocked waiting for a capture to complete. This protects the "friction-free" guarantee -- if the queue is broken, the user should not notice.

### 2. Write-Only (No Readback)

The UI is strictly write-only in v1. There is no command, endpoint, or surface to read back captured events. Data flows in one direction: user to queue to sink. SMITH handles all downstream consumption.

### 3. No Categorization

Tarcie does not categorize, tag, or group events beyond extracting a literal `#tag` string from note content. All semantic grouping, trend analysis, and categorization is the responsibility of SMITH.

### 4. No AI / No LLMs

Tarcie processes raw strings only. There is no AI, no LLM, no inference, no embeddings, no summarization. Content is captured verbatim and flushed verbatim.

### 5. Small, Non-Blocking UI

The overlay window is 480x140px. It must never block other applications. It appears on hotkey, accepts input, and disappears. No modal dialogs, no confirmation prompts, no settings screens.

## Constants

All defined in `constraints.rs`:

| Constant | Value | Purpose |
|----------|-------|---------|
| `SOURCE_VERSION` | `"tarcie-v1.0.0"` | Stamped on every event |
| `MAX_CONTEXT_CHARS` | 64 | Max length of `app_context` field |
| `MAX_TAG_CHARS` | 32 | Max length of extracted `#tag` |
| `MAX_CONTENT_BYTES` | 10,240 (10 KB) | Max size of `content` field |
| `DEFAULT_FLUSH_INTERVAL_SECS` | 300 | Background flush timer |
| `DEFAULT_BATCH_MAX` | 200 | Events per HTTP POST |
| `DEFAULT_QUEUE_MAX_EVENTS` | 10,000 | Queue cap before rotation |
| `HOTKEY_DEBOUNCE_MS` | 500 | Minimum interval between hotkey activations |

---

# 10. Testing

## Current State

Tarcie has a unit test suite. The tests live beside the code they cover, in
`#[cfg(test)]` modules in `queue/jsonl.rs`, `ipc/commands.rs`, `sink/config.rs`,
and `flusher.rs`.

The suite covers the seven priority areas:

| Area | Module | Proves |
|------|--------|--------|
| Append and read round-trip | `queue/jsonl.rs` | An appended event reads back with its fields intact, and order holds |
| Tolerant read | `queue/jsonl.rs` | A malformed, truncated, or blank line is skipped; valid events still return |
| Cap rotation | `queue/jsonl.rs` | An append at the cap rotates the file first, and keeps every capped event |
| Content clamping | `ipc/commands.rs` | Oversized content is clamped, not rejected, and stays valid UTF-8 |
| Tag extraction | `ipc/commands.rs` | `#tag` becomes the context; absent tags fall back to the default |
| Sink URL validation | `sink/config.rs` | A remote sink is refused unless the operator opts in |
| FlushResult variants | `flusher.rs` | `Empty`, `Success`, and `Deferred` each occur, and a deferral keeps every event |

Some tests pin behavior that deviates from the documented intent. Each one
carries a `KNOWN DEVIATION` comment that states the deviation. These tests
record what the code does today. They do not endorse it. A future fix must
change the test in the same commit.

## Prerequisites

`cargo test` builds the full Tauri binary, so it needs the frontend bundle and
the Linux system libraries.

```bash
npm install && npm run build   # creates dist/, required by frontendDist
```

Without `dist/`, the build fails in `tauri::generate_context!`.

On Debian or Ubuntu, the system libraries are:

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev \
  build-essential pkg-config
```

## Building

```bash
cd src-tauri
cargo build
```

## Running in Development

```bash
cd src-tauri
cargo tauri dev
```

This launches the Tauri development server with hot-reload for the frontend.

## Running Tests

```bash
cd src-tauri
cargo test
```

## Type Checking

```bash
cd src-tauri
cargo check
```

## Lint

```bash
cd src-tauri
cargo clippy -- -W clippy::all
```

## Not Yet Covered

The seven priority areas are covered. These areas are not:

1. **The Tauri command layer.** `capture_note`, `capture_marker`, and
   `flush_now` need a Tauri `State`, so the tests cover the logic they call
   instead of the commands themselves.
2. **Concurrent append during a flush.** The flusher reads, posts, and then
   rotates. An append between the read and the rotate is filed as sent. No
   test covers this window yet.
3. **Multi-batch flush.** A flush that succeeds on one batch and fails on a
   later one re-sends the succeeded batch on the next cycle.
4. **Rotation timestamp collisions.** Rotation names files to the second. Two
   rotations in one second overwrite each other.
5. **The global hotkey and window toggle.** These need a running desktop
   session.
6. **Device ID persistence.** `load_or_create_device_id` writes to the real
   user profile and has no path seam.

---

# 11. Handover

## Implementation Status

**Tarcie v1.0.0** -- stable, feature-complete for v1 scope.

All modules implemented: IPC commands, JSONL queue, HTTP sink client, background flusher, data model, constraints, state management, platform paths, global hotkey.

## Critical Constraints (Do Not Violate)

1. **Write-only.** No readback surfaces. No query commands. No browsing UI.
2. **No AI.** Raw strings only. No LLMs, no embeddings, no summarization.
3. **No categorization.** SMITH does grouping. Tarcie captures verbatim.
4. **Localhost-only default.** Remote sink requires explicit `TARCIE_ALLOW_REMOTE_SINK=true`.
5. **5-second capture revert.** If capture takes > 5s, revert. Never block the user.

## Known Limitations

- **Test coverage is unit-level only.** The seven priority areas in section 10
  have tests. The Tauri command layer, the hotkey, and device-ID persistence do
  not. Section 10 lists what stays uncovered.
- **A flush can file an unsent capture as sent.** The flusher reads the queue,
  posts it, and then rotates the file. An append between the read and the
  rotate goes to `queue.sent.*` without being transmitted.
- **A multi-batch flush can send a batch twice.** If an early batch succeeds
  and a later one fails, the flush defers without rotating, so the succeeded
  batch is posted again on the next cycle.
- **Rotation file names resolve to the second.** Two rotations within one
  second overwrite each other.
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

---

# Operations

**Document version:** 1.0 (bootstrap scaffold)

Deployment, observability, incident response, and bounded repair.

> This chapter is a registry-generated bootstrap scaffold for a
> `application` class documentation system. Replace this placeholder with
> real authored content. Registry will not invent repo truth that is not
> already present in the repo.

---

# Appendices

**Document version:** 1.0 (carry-forward)

Appendices, glossary, and cross-references.

## Unmapped legacy chapters

The following legacy chapters were carried forward but could not be
deterministically mapped to a class-aware slot. Review and place them by
hand:

- `Tarcie — System Documentation`
- `3. Command Reference`
- `5. Queue System`
- `6. Flush Pipeline`
- `7. Configuration`
- `8. Constraints`
- `9. Error Handling`
- `10. Testing`
- `11. Handover`
- `Build`
- `Run in dev mode`
- `Check types`
- `Lint`
- `Environment overrides`

---

# 1. Overview

Tarcie is a friction-free capture tool for notes and markers. A global hotkey (`Ctrl+Alt+T`) pops a 480x140px overlay window. The user types a note or drops a timestamp marker. The event is appended to a JSONL file queue with fsync durability, and a background flusher periodically batches events to an HTTP sink endpoint.

## Design Philosophy

Tarcie is strictly **write-only** in v1. There is no readback surface, no categorization logic, and no AI processing. Raw strings go in, raw strings go out. SMITH handles grouping and analysis downstream.

## At a Glance

| Metric | Value |
|--------|-------|
| Rust LOC | 636 |
| IPC Commands | 3 |
| Frontend | Vanilla TypeScript (main.ts + styles.css + index.html) |
| Framework | Tauri 2.0 (no SvelteKit, no UI framework) |
| Edition | Rust 2024 |
| Version | 1.0.0 |

## What Tarcie Does

1. Listens for `Ctrl+Alt+T` global hotkey
2. Shows/hides a minimal overlay window
3. Accepts text notes (with optional `#tag`) or timestamp markers
4. Appends events to a local JSONL queue (fsync-durable)
5. Flushes queued events to an HTTP sink in batches on a timer
6. Attempts a graceful flush on shutdown (5-second timeout)

## What Tarcie Does Not Do

- No readback or browsing of captured events
- No categorization, tagging intelligence, or grouping
- No AI/LLM processing of any kind
- No complex UI beyond the capture overlay

---

---

# 2. Architecture

## System Diagram

```
  User
   │
   │  Ctrl+Alt+T
   ▼
┌──────────────────────┐
│  Overlay Window       │  480x140px, vanilla TS
│  (main.ts + index.html)│
└──────────┬───────────┘
           │ Tauri IPC
           ▼
┌──────────────────────┐
│  IPC Commands         │
│  ├─ capture_note()    │
│  ├─ capture_marker()  │
│  └─ flush_now()       │
└──────────┬───────────┘
           │ TarcieEvent
           ▼
┌──────────────────────┐
│  JSONL Queue          │  Mutex-protected, fsync append
│  queue.jsonl          │
└──────────┬───────────┘
           │ Background timer
           ▼
┌──────────────────────┐
│  Flusher              │  Batch POST, exp. backoff
│  ├─ Read queue        │
│  ├─ Chunk into batches│
│  └─ POST to sink      │
└──────────┬───────────┘
           │ HTTP POST
           ▼
┌──────────────────────┐
│  HTTP Sink            │  default: 127.0.0.1:8080
│  /ingest/tarcie       │  localhost-only by default
└──────────────────────┘
```

## Module Map

| Module | Files | Purpose |
|--------|-------|---------|
| `ipc/` | `commands.rs`, `mod.rs` | 3 Tauri IPC commands |
| `queue/` | `jsonl.rs`, `mod.rs` | JSONL file queue (append, read, rotate) |
| `sink/` | `client.rs`, `config.rs`, `mod.rs` | HTTP sink client + env-based config |
| `flusher.rs` | -- | Background flush loop with retry and batch posting |
| `model.rs` | -- | `TarcieEvent` struct + `EventType` enum |
| `constraints.rs` | -- | All v1 hard limits and constants |
| `state.rs` | -- | `AppState` (config, queue, flusher, device_id, mono_start) |
| `util/` | paths module | Platform directory paths via `directories` crate |

## Data Flow

1. User presses `Ctrl+Alt+T` -- overlay toggles visibility
2. User types text and submits -- frontend calls `capture_note` or `capture_marker` via Tauri IPC
3. Command builds a `TarcieEvent` with UUID, device ID, UTC + monotonic timestamps
4. Event is serialized to JSON and appended to `queue.jsonl` with fsync
5. Background flusher wakes on interval (default 300s) or manual `flush_now`
6. Flusher reads all events, batches them (max 200 per batch), POSTs to sink
7. On success: queue file rotated to `queue.sent.TIMESTAMP.jsonl`
8. On failure after retries: events remain in queue for next attempt

## Dependencies

| Crate | Purpose |
|-------|---------|
| `tauri` 2.x | Desktop application framework |
| `tauri-plugin-global-shortcut` | `Ctrl+Alt+T` hotkey registration |
| `serde` + `serde_json` | Serialization |
| `uuid` | Event and device ID generation |
| `chrono` | UTC timestamps |
| `directories` | Platform-appropriate file paths |
| `tokio` | Async runtime |
| `reqwest` (rustls-tls) | HTTP client for sink |
| `anyhow` | Error handling |
| `regex` | Tag extraction from note content |
| `url` | Sink URL validation |

---
