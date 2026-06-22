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
