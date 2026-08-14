# Tarcie

**Friction-free capture tool for notes and markers**

Built with Tauri v2, Rust, and TypeScript

---

## Documentation Contract

- **Repo type:** Desktop/local capture tool
- **Authority boundary:** Fast local note and marker capture with downstream sink handoff; not a resident ecosystem service and not the durable truth store
- **Deep reference:** `doc/system/_index.md`, `doc/TARSYSTEM.md`, `../docs/canonical/documentation_protocol_v1.md`
- **README role:** Product overview and local run entrypoint
- **Truth note:** Feature lists, version labels, and implementation notes in this README are snapshot facts unless explicitly marked as canonical doctrine or target values

## Overview

Tarcie is a minimal, always-ready capture tool designed for sub-5-second note entry. It runs in the background, activates via global hotkey, queues entries locally in crash-proof JSONL format, and flushes them to a configurable sink endpoint.

Tarcie is **write-only by design**. It captures raw strings without categorization, AI processing, or readback surfaces. Downstream systems (like SMITH) handle grouping and analysis.

---

## Core Constraints

1. **If capture takes > 5s, REVERT** — Speed is non-negotiable
2. **Write-only from UI** — No readback surfaces in v1
3. **No categorization logic** — SMITH does grouping
4. **No AI / LLMs** — Raw strings only
5. **UI must be small and non-blocking** — 480x140px overlay

---

## Features

- **Global Hotkey** — `Ctrl+Alt+T` toggles the capture window from anywhere
- **Instant Capture** — Type a note, press Enter, window hides automatically
- **Markers** — One-click timestamp markers for significant moments
- **Tag Extraction** — Optional `#tag` syntax for context (e.g., `#meeting discuss roadmap`)
- **Crash-Proof Queue** — JSONL with fsync ensures no data loss
- **Background Flush** — Automatic sync to sink endpoint with retry logic
- **Localhost-Only Default** — Remote sinks require explicit opt-in

---

## Installation

### From .deb Package (Debian/Ubuntu)

```bash
sudo dpkg -i tarcie_1.0.0_amd64.deb
```

### From Source

**Prerequisites:**
- Node.js 20+
- Rust 1.77+
- pnpm or npm

```bash
# Clone and enter directory
cd tarcie

# Install frontend dependencies
npm install

# Development mode
npm run tauri dev

# Production build
npm run tauri build -- --bundles deb
```

---

## Usage

### Capture a Note

1. Press `Ctrl+Alt+T` to show the window
2. Type your note (optional: prefix with `#tag`)
3. Press `Enter` to capture
4. Window auto-hides with green flash confirmation

### Examples

```
Fix the login timeout bug
#meeting Decided to use PostgreSQL
#idea What if we added dark mode?
#bug Users can't reset password on mobile
```

### Capture a Marker

Click the red button to insert a timestamp marker without text. Useful for:
- Marking significant moments during recordings
- Timestamping events for later review
- Creating reference points in workflows

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Ctrl+Alt+T` | Toggle window (global) |
| `Enter` | Capture note and hide |
| `Escape` | Hide without capturing |

---

## Configuration

Tarcie is configured via environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `TARCIE_SINK_URL` | `http://127.0.0.1:8080/ingest/tarcie` | Endpoint for flushing events |
| `TARCIE_SINK_AUTH` | *(none)* | Auth header value (auto-prefixed with `Bearer` if needed) |
| `TARCIE_ALLOW_REMOTE_SINK` | `false` | Set to `true` to allow non-localhost sinks |
| `TARCIE_FLUSH_INTERVAL_SECS` | `300` | Background flush interval (5 minutes) |
| `TARCIE_BATCH_MAX` | `200` | Maximum events per flush batch |
| `TARCIE_QUEUE_MAX_EVENTS` | `10000` | Queue rotation threshold |

### Example Configuration

```bash
export TARCIE_SINK_URL="http://127.0.0.1:8080/ingest/tarcie"
export TARCIE_FLUSH_INTERVAL_SECS="60"
export TARCIE_BATCH_MAX="100"
```

---

## Data Model

### TarcieEvent

Each captured note or marker is stored as a `TarcieEvent`:

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "device_id": "123e4567-e89b-12d3-a456-426614174000",
  "timestamp_utc": "2024-01-15T10:30:00Z",
  "timestamp_mono_ms": 45000,
  "event_type": "Note",
  "content": "Fix the login timeout bug",
  "app_context": "General",
  "source_version": "tarcie-v1.0.0"
}
```

### Event Types

- **Note** — Text content with optional tag context
- **Marker** — Timestamp-only event with optional reason

### Tag Extraction

Tags are extracted from content and moved to `app_context`:

| Input | Content | Context |
|-------|---------|---------|
| `#meeting discuss roadmap` | `discuss roadmap` | `meeting` |
| `Fix the bug` | `Fix the bug` | `General` |
| `#idea #backup first tag wins` | `#backup first tag wins` | `idea` |

---

## Architecture

```
tarcie/
├── src/                    # Frontend (TypeScript)
│   ├── index.html          # Minimal capture UI
│   ├── main.ts             # Tauri API integration
│   └── styles.css          # Dark theme styling
│
├── src-tauri/              # Backend (Rust)
│   ├── src/
│   │   ├── main.rs         # App setup, hotkey, background flush
│   │   ├── constraints.rs  # Core constants and limits
│   │   ├── model.rs        # TarcieEvent, EventType
│   │   ├── state.rs        # Shared AppState
│   │   ├── flusher.rs      # Retry-aware flush logic
│   │   ├── ipc/
│   │   │   └── commands.rs # IPC: capture_note, capture_marker, flush_now
│   │   ├── queue/
│   │   │   └── jsonl.rs    # Append-only JSONL with fsync
│   │   ├── sink/
│   │   │   ├── config.rs   # Environment-driven config
│   │   │   └── client.rs   # HTTP client for sink
│   │   └── util/
│   │       └── paths.rs    # App data directory paths
│   └── tauri.conf.json     # Tauri configuration
│
└── package.json            # Frontend build config
```

### Data Flow

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│   User      │────▶│   Tarcie     │────▶│  JSONL      │
│   Input     │     │   Window     │     │  Queue      │
└─────────────┘     └──────────────┘     └──────┬──────┘
                                                │
                                                ▼
                                         ┌─────────────┐
                                         │  Background │
                                         │  Flusher    │
                                         └──────┬──────┘
                                                │
                                                ▼
                                         ┌─────────────┐
                                         │   Sink      │
                                         │  Endpoint   │
                                         └─────────────┘
```

### Queue Strategy

- **JSONL append-only** with atomic fsync per write
- **Tolerant parsing** — Skips malformed lines, doesn't brick on corruption
- **Claim before delivery** — A flush renames `queue.jsonl` into `sending/`
  before it posts anything, so an event captured mid-flush lands in a fresh
  queue and is never archived as sent without being sent
- **Retry only what is owed** — A flush that fails partway writes back just the
  undelivered remainder, so an accepted batch is not resent
- **Crash recovery** — A batch left in `sending/` by an interrupted flush is
  picked up by the next claim
- **Growth cap** — Rotates at 10,000 events (configurable)

### Storage Locations

| Platform | Path |
|----------|------|
| Linux | `~/.local/share/tarcie/` |

```
~/.local/share/tarcie/
├── device_id.txt           # Persistent device UUID
└── queue/
    ├── queue.jsonl         # Active queue — appends land here
    ├── sending/            # Claimed by a flush, awaiting delivery
    │   └── 20260814T103000Z-000004.jsonl
    └── sent/
        └── queue.sent.20260814T103000Z-000005.jsonl
```

File stamps carry a sequence number after the timestamp. The timestamp resolves
to the second, so without it two rotations in the same second collide and one
file is lost.

---

## Security

### Attack Surface Minimization

- **3 IPC commands only** — `capture_note`, `capture_marker`, `flush_now`
- **No shell execution** — No arbitrary command paths
- **No arbitrary FS access** — Only writes to app data directory
- **Localhost-only by default** — Remote sinks blocked unless explicitly enabled

### Input Validation

| Field | Constraint |
|-------|------------|
| Content | Max 10KB, truncated with `[…truncated]` marker |
| Tag | Regex `[a-zA-Z0-9_-]{1,32}`, invalid tags ignored |
| Context | Max 64 characters |

### Remote Sink Protection

```bash
# This will FAIL by default:
export TARCIE_SINK_URL="https://remote-server.com/ingest"

# Must explicitly opt-in:
export TARCIE_ALLOW_REMOTE_SINK="true"
```

---

## API

### Sink Endpoint Contract

Tarcie POSTs to the configured sink with:

```http
POST /ingest/tarcie HTTP/1.1
Content-Type: application/json
User-Agent: tarcie-v1
Authorization: Bearer <token>  # if TARCIE_SINK_AUTH set

{
  "source": "tarcie",
  "events": [
    {
      "id": "...",
      "device_id": "...",
      "timestamp_utc": "...",
      "timestamp_mono_ms": 45000,
      "event_type": "Note",
      "content": "...",
      "app_context": "...",
      "source_version": "tarcie-v1.0.0"
    }
  ]
}
```

**Expected Response:** HTTP 2xx on success. Non-2xx triggers retry with exponential backoff.

---

## Development

### Commands

```bash
# Development with hot reload
npm run tauri dev

# Run the tests — needs dist/, so build the frontend first
npm install && npm run build
cd src-tauri && cargo test

# Rebuild the system document after changing doc/system/
bash doc/system/BUILD.sh

# Type check Rust
cd src-tauri && cargo check

# Build release
npm run tauri build

# Build .deb only
npm run tauri build -- --bundles deb

# Build AppImage only
npm run tauri build -- --bundles appimage
```

### Testing the Sink

Run a simple test server:

```bash
# Python
python3 -c "
from http.server import HTTPServer, BaseHTTPRequestHandler
import json

class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers['Content-Length'])
        data = json.loads(self.rfile.read(length))
        print(json.dumps(data, indent=2))
        self.send_response(200)
        self.end_headers()

HTTPServer(('127.0.0.1', 8080), Handler).serve_forever()
"
```

### Manual Flush

Trigger an immediate flush from the app or via IPC:

```typescript
import { invoke } from "@tauri-apps/api/core";
const result = await invoke("flush_now");
// Returns: "empty" | "ok:N" | "deferred:reason"
```

---

## Troubleshooting

### Window doesn't appear on hotkey

1. Check if another app is using `Ctrl+Alt+T`
2. On some Linux DEs, the shortcut may conflict with terminal

### Events not flushing

1. Check if sink is running: `curl http://127.0.0.1:8080/ingest/tarcie`
2. Check queue file: `cat ~/.local/share/tarcie/queue/queue.jsonl`
3. Review logs in terminal if running in dev mode

### Permission denied on Linux

```bash
# Ensure app data directory is writable
mkdir -p ~/.local/share/tarcie
chmod 755 ~/.local/share/tarcie
```

---

## Roadmap

- [ ] System tray icon with status indicator
- [ ] Configurable hotkey
- [ ] Multiple sink endpoints
- [ ] Offline-first sync with conflict resolution
- [ ] Optional encryption at rest

---

## License

MIT

---

## Related Projects

- **Forge:SMITH** — Governance-enforced AI engineering workbench (downstream consumer)
- **DataForge** — Persistent state and analytics platform
