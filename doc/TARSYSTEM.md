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
| `util/` | `paths.rs`, `device.rs`, `log.rs` | Platform directory paths via `directories` crate, the device identity, and the operational log |

The frontend is three modules: `main.ts` finds the elements and hands
`overlay.ts` the real Tauri calls, `overlay.ts` wires the gestures, and
`capture.ts` holds the decisions apart from the DOM and from Tauri.

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

Each command needs a Tauri `State`, which a test cannot supply. Each one
therefore adds only the state extraction and delegates to a function over a
plain `&AppState` — `capture_note_into`, `capture_marker_into`, `flush_now_on`.
Those functions hold the behaviour described here.

## capture_note

```rust
#[tauri::command]
pub async fn capture_note(
    content: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String>
```

**Purpose:** Capture a text note.

**Behavior:**

1. Clamp `content` to `MAX_CONTENT_BYTES` (10 KB), which trims it
2. Refuse the note unless it says something that is not a tag, before anything
   is written
3. Extract the first `#tag` as `app_context`, clamped to `MAX_TAG_CHARS` (32)
4. Build a `TarcieEvent` with:
   - Fresh UUID
   - Device ID from state
   - UTC timestamp + monotonic offset
   - `EventType::Note`
5. Append the event to the JSONL queue (fsync-durable)

**Returns:** `Ok(())`. There is no success payload.

**Refusals and errors:** A note that says nothing of its own is refused, and
nothing is written. A queue write failure returns the stringified error.

Step 2 is a guard on the queue rather than on the user. An event with no text is
as durable as any other — queued, delivered, and archived for good — so the
cheapest place to stop one is before it is written.

The check takes **every** tag out of the text and asks whether anything is left.
An empty box, whitespace, `#bug`, and `#a #b` are all refused. A tag names the
context an observation belongs to, and with no observation there is nothing to
place under it.

Removing every tag is only how the decision is made. Extraction is unchanged:
step 3 still takes the first tag as the context and leaves any others in the
content.

`has_text_of_its_own` and `extract_tag` share one tag pattern through `tag_re`,
so the rule and the extraction cannot drift apart.

This command is the one place that decides what counts as a note. The overlay
stops an obviously empty box before anything is sent and stops at that, which
keeps the tag pattern in one place rather than two.

---

## capture_marker

```rust
#[tauri::command]
pub async fn capture_marker(
    reason: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String>
```

**Purpose:** Drop a timestamp marker event.

**Behavior:**

1. Clamp `reason` to `MAX_CONTENT_BYTES` if one was given, which trims it
2. Read the label the way a note is read: the first `#tag` becomes
   `app_context`, and whatever is left stays as the `reason`
3. Build a `TarcieEvent` with:
   - Fresh UUID
   - Device ID from state
   - UTC timestamp + monotonic offset
   - `EventType::Marker { reason }`
   - An empty `content`
4. Append the event to the JSONL queue (fsync-durable)

**Returns:** `Ok(())`. There is no success payload.

**Errors:** Returns the stringified error on a queue write failure.

**A marker needs no text of its own**, which is where it parts company with a
note. The gesture is the observation, so the check that guards `capture_note`
does not apply here. A marker with no reason at all is whole, and so is one
labelled only `#bug`: it says a moment matters and names what it belongs to.
That is the shape a tag-only note used to have before notes began refusing it.

| Reason in | `app_context` | `reason` stored |
|---|---|---|
| *(none)* | `General` | *(none)* |
| `#bug` | `bug` | *(none)* |
| `#bug the overlay froze` | `bug` | `the overlay froze` |
| `stepping away` | `General` | `stepping away` |

The overlay sends whatever is in the box as the label, so the same typing
produces a note under Enter and a marker under the button.

---

## flush_now

```rust
#[tauri::command]
pub async fn flush_now(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String>
```

**Purpose:** Trigger an immediate flush of the queue to the sink.

**Behavior:**

1. Take the flush lock
2. **Claim** the queue — `queue.jsonl` moves into `sending/`, with any batch an
   interrupted flush left there. Section 5 describes why
3. Batch the claimed events and POST them to the sink
4. Complete or defer the claim, and return one of:
   - `"empty"` — the claim held no events
   - `"ok:N"` — N events were delivered
   - `"deferred:reason"` — delivery stopped, and every undelivered event is kept

**Errors:** Returns the stringified error on unexpected failures.

The deferral reason carries the whole cause chain, not just the attempt.
Section 6 describes the loop, the per-request bound, and the retry strategy.

---

# Product Surface

Everything a person can do with tarcie, and everything tarcie says back.

The surface is one window with one text box, one button, and four gestures.
There is nothing else: no menu, no tray icon, no settings screen, no history,
and no notifications.

## Entry

The global hotkey `Ctrl+Alt+T` is the only way in. It toggles the overlay:
visible becomes hidden, hidden becomes visible and focused.

The window is created hidden (`"visible": false`), skips the taskbar, and has no
tray icon, so nothing on screen offers a way to open it. A hotkey that the
operating system refuses to grant therefore leaves tarcie unreachable. Section
10 records that the binding is proven to parse and to name the combination the
code registers, and that whether the system grants it is not covered.

Repeated presses inside `HOTKEY_DEBOUNCE_MS` (500 ms) are ignored, so a key that
repeats does not flicker the window.

## The window

From `src-tauri/tauri.conf.json`:

| Property | Value | What it means on screen |
|---|---|---|
| `width` × `height` | 480 × 140 | Constraint 5: small enough not to take over |
| `resizable` | `false` | One size; there is nothing to lay out |
| `alwaysOnTop` | `true` | It sits over the work being observed |
| `skipTaskbar` | `true` | It does not appear as a running window |
| `visible` | `false` | It starts hidden and waits for the hotkey |
| `center` | `true` | It arrives in the same place every time |
| `decorations` | `true` | It keeps a title bar, and therefore a close button |
| `title` | `Tarcie` | — |

**The close button is not the Escape key.** Escape hides the overlay. Closing
the window ends the application, after a final flush bounded by
`SHUTDOWN_FLUSH_SECS`. Section 6 describes that flush.

## What is on it

Three elements, in `src/index.html`:

| Element | Id | Appearance |
|---|---|---|
| Text box | `tarcie-input` | Placeholder `Type one friction note… (optional #tag)`, focused on arrival |
| Marker button | `tarcie-marker` | A red circle, titled `Marker` |
| Status | `tarcie-status` | Empty except during a confirmation |

## The four gestures

| Gesture | What it does |
|---|---|
| `Ctrl+Alt+T` | Shows the overlay, focused, or hides it |
| `Enter` | Captures the text in the box as a note |
| Marker button | Captures a marker, labelled with whatever is in the box |
| `Escape` | Hides the overlay and captures nothing |

Section 3 states what each capture becomes, including which inputs are refused
and how a `#tag` is read.

`Escape` leaves the text where it is. The overlay is hidden rather than
destroyed, so an unsent draft is still in the box at the next hotkey press. It
survives until the application exits.

## What tarcie says back

One word, once, and only when a capture is confirmed.

The body takes a green outline, the status reads `Captured`, and both last
`FLASH_MS` (200 ms). The overlay then hides, and the box is cleared if that
capture took its text.

That is the whole vocabulary. There is no progress indicator, no error dialog,
no failure state, and no sink or queue status anywhere on screen.

**Silence is the other half of it.** A refused capture, a capture that outlived
its five-second budget, and a gesture turned away by a guard all look
identical: nothing changes. The overlay stays open, holding the text. The window
that did not go away is the whole signal, and the text still on screen is the
only copy anybody can point to. Section 9 sets this out.

## What is deliberately not here

- **No readback.** Nothing displays, searches, edits, or exports what was
  captured. Constraint 2 makes this a boundary rather than a gap: a "show me
  what I captured" surface is a scope change.
- **No settings screen.** Every setting is an environment variable, read once at
  startup. Section 7 lists them.
- **No status surface.** Whether delivery is working is reported to a log file,
  never to the overlay. Section 9 describes the log.
- **No account, no sync, no sharing.** Tarcie captures and forwards.

## The webview

The overlay renders no captured text as HTML. The box holds what the user
typed, the status holds one fixed word, and nothing from the queue or the sink
is ever displayed — which follows from there being no readback at all.

`"csp": null` in the window's security block disables the webview content
security policy. No injection path exists today, because no untrusted content
reaches the page. A surface that displays anything captured, or anything a sink
returns, would change that and needs the policy settled first.

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
4. `fsync` the file
5. `fsync` the queue directory, when this append created the file

All appends are protected by a `Mutex` to prevent interleaved writes from concurrent IPC calls.

Step 5 is the other half of step 4. An `fsync` on a file covers its contents.
It says nothing about the directory entry that names the file, so after a power
loss the data can be on the disk with nothing pointing at it. For a newly
created `queue.jsonl` that is every capture in it.

Only the append that creates the file pays for the second sync, so the cost
falls once per flush cycle rather than once per capture.

## Read (Tolerant)

The queue reader is tolerant of malformed lines:

- Each line is attempted as JSON deserialization
- Malformed lines are skipped with a warning (not fatal)
- Processing continues to the next line

This ensures a single corrupted event never blocks the entire queue.

## Claim

A flush does not read the live queue and delete it afterwards. It **claims**
the queue first: `queue.jsonl` is renamed into `sending/` under the same lock
that guards `append`.

The rename is the handoff. An event captured while a flush is in flight lands
in a fresh `queue.jsonl` and is never part of the claim, so it cannot be
archived as sent without being sent.

A claim also picks up every file already in `sending/`, oldest first. A flush
that was interrupted leaves its batch there, and the next claim recovers it.
The cost of a crash is a retry, not a capture.

The claim ends one of two ways:

- **Complete.** Every event reached the sink. Each claim file is renamed to
  `queue.sent.{STAMP}.jsonl` in the sent directory.
- **Defer.** Only the first *n* events reached the sink. The remainder is
  written back into `sending/` and the originals are archived. The next flush
  retries what is still owed and does not resend what the sink accepted.

If the process dies between writing the remainder and archiving the originals,
the remainder is delivered twice. The reliability contract prefers a duplicate
to a loss.

## Cap Rotation

When the queue reaches `DEFAULT_QUEUE_MAX_EVENTS` (10,000 events), the current
`queue.jsonl` moves into `sending/` as:

```
{STAMP}.cap.jsonl
```

A fresh `queue.jsonl` is created for new events. This keeps any one file from
growing without limit while the sink is unreachable.

The stamp comes first in the name, so a capped batch sorts into place by age
among the claimed ones. The next claim picks it up and delivers it along with
everything else.

The capped file used to move into the sent directory instead. No claim reads
that directory, so every event in a capped file was discarded without a word —
in the one situation the durable queue exists for. Cap rotation bounds the size
of a file. It never decides an event is not worth sending.

## Stamps

`{STAMP}` is a UTC timestamp to the second followed by a per-process sequence
number, for example `20260814T033500Z-000004`.

The sequence number is not decoration. The timestamp alone resolves to the
second, so two rotations within one second produced the same name and the
second rename destroyed the first file.

The sequence counts from zero again when the process starts. A run that begins
in the same second as a crashed one can therefore build a name that run already
used — an orphan left in `sending/`. `rename` replaces such a file without a
word, and the events in it would be gone.

Every stamped path is therefore taken through `free_path`, which checks the
name is free and takes a fresh stamp if it is not. A retry sorts after the name
it could not have, so claim order still follows the clock. When no free name
turns up, the placement fails: a failed flush leaves every event queued for the
next one, which the reliability contract prefers to an overwrite.

## Durable Placement

Every rename in the queue is a handoff of custody, and a rename that has not
reached the disk is one a power loss can undo. The same four callers that take
their names through `free_path` — claim, defer, archive, and cap rotation — put
the file in place through `rename_durably`.

It syncs both directories: the one that gains the name, so the events are
findable under it, and the one that loses it, so the old name cannot come back
and offer the same events a second time.

Windows cannot open a directory as a file and has no equivalent call. Tarcie
qualifies on Linux first, so the sync is a no-op there rather than a failure.

## Retention

The sent directory is never pruned. Nothing in tarcie deletes a file, so every
event ever captured stays on the disk under `queue/sent/` after it has been
delivered, for the life of the installation.

Two consequences follow, and neither is yet an operator decision that has been
taken:

- **Disk.** The archive grows without limit. Cap rotation bounds the size of
  one file and nothing bounds the total.
- **Retention.** A write-only capture tool keeps a complete plain-text copy of
  everything the user has captured. Section 8 records that there is no
  encryption at rest, which this compounds.

Whether the archive is a safety net worth its cost, or should age out, is a
decision for the operator. Tarcie does not take it, and this section exists so
that the decision is made rather than inherited.

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
3. **Claim** the queue — `queue.jsonl` moves into `sending/`, and any batch left
   by an interrupted flush is picked up with it
4. If the claim is empty, archive its files and return to step 1
5. Chunk the claimed events into batches of `DEFAULT_BATCH_MAX` (200)
6. POST each batch to the sink endpoint, counting what is accepted
7. On full success: archive the claim to `queue.sent.{STAMP}.jsonl`
8. On failure partway: **defer** — write the undelivered remainder back to
   `sending/`, archive the originals, and return `Deferred`
9. Release lock, return to step 1

Step 3 is what keeps a capture safe. The queue is moved aside before anything
is posted, so an event captured during delivery is not in the claim and cannot
be archived as sent.

Step 8 is what keeps delivery honest. A flush that accepted three batches and
failed on the fourth retries only the fourth; the three the sink already holds
are not offered again.

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
- **Per request:** `SINK_REQUEST_TIMEOUT_SECS` (30s) bounds one POST
- **On exhaustion:** Flush returns `Deferred` with reason. Events remain in the queue file untouched

The per-request bound is what ends a wait on a sink that stops answering. A
refused connection fails at once and reports itself. A sink that accepts the
connection and then goes quiet holds a healthy connection open and reports
nothing, and `reqwest` applies no time bound of its own.

The bound matters because the background flusher is a single task. A flush that
does not return takes every later flush with it, for the rest of the session.
Captures still reach the queue, nothing leaves it, and the log stays silent,
because a deferral is only logged once a flush ends.

Four attempts and the backoff between them come to 134 seconds. That is inside
the 300-second default flush interval, so a bounded flush finishes before the
next one is due.

## FlushResult

The flusher returns one of three outcomes:

| Result | Meaning |
|--------|---------|
| `Empty` | Queue had no events to flush |
| `Success(n)` | Successfully flushed `n` events, queue rotated |
| `Deferred(reason)` | Flush failed after retries, events remain in queue |

## Manual Flush

The `flush_now` IPC command triggers an immediate flush cycle outside the timer. It follows the same logic as the background loop but returns the result directly to the caller.

## Reporting

The background loop logs a `Deferred` result with its reason. A deferral is the
queue keeping its promise rather than a fault, but it is also the only word
anyone gets that captures are not arriving: tarcie has no readback surface, so
an unreported deferral leaves a sink that has been refusing for days looking
like a sink with nothing to do.

The reason carries the whole cause chain. `anyhow` prints only the outermost
context in its plain `Display`, and every failure inside `post_json` carries the
context `POST to sink`. A refused connection, a timeout, and a name that does
not resolve therefore reported the same four words, which name the attempt and
never the cause. `MAX_LOG_LINE_CHARS` still bounds the line.

`Empty` and `Success` are not logged. A flush that worked has nothing to say.

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

The reason names the cause, not the attempt. It carries the whole error chain,
because `anyhow` prints only the outermost context on its own and every failure
in `post_json` shares one. A sink that answers with an error status already
names itself, because that path builds its own message.

## Queue Read Tolerance

The JSONL reader skips malformed lines rather than failing the entire read. This means:

- A single corrupted line does not block queue processing
- Malformed lines are logged as warnings
- All valid events in the file are still processed

This is intentional: data durability of valid events is prioritized over strict consistency.

## The Log

Tarcie has no readback surface, so nothing on screen reports that delivery has
stopped. Everything the application has to say goes to a file instead:

| File | Path |
|------|------|
| Live log | `<data dir>/logs/tarcie.log` |
| Previous log | `<data dir>/logs/tarcie.1.log` |

`<data dir>` is the platform data directory the `directories` crate resolves,
through `util::paths::logs_dir`.

The log opens first in `setup`, so every step after it can report. A log that
will not open is not a reason to refuse to capture: the reports fall back to
stderr and the application carries on. Reports go to stderr in any case, so a
run from a terminal still shows them.

### What it holds

- the sink and the flush interval, at startup
- a deferred flush, with its reason
- a background flush error
- a queue line that did not parse, by line number
- a queue that reached its cap
- a device ID file that could not be read

**No capture content is ever written to the log.** The log records what happened
to events, never what was in them. A line that carries content is a defect: the
log sits outside the queue, so content in it is a copy of the user's notes that
nothing in the design accounts for.

The sink URL is reported through `url_without_credentials`, because a URL can
carry a username and password and `Url`'s own `Display` prints them. The auth
token is never reported.

### Bounds

The log is capped at `MAX_LOG_BYTES`. On reaching it the file is renamed to
`tarcie.1.log` and a fresh one starts, so the pair costs at most twice the cap.

Replacing the previous log is deliberate, and is the one rename in tarcie
allowed to overwrite. The queue never overwrites, because an overwritten queue
file is lost captures. An overwritten log is lost diagnostics, and a log that
grows until the disk is gone would take the queue with it.

One line is capped at `MAX_LOG_LINE_CHARS`. A deferral carries the sink's
response text, and how long that runs is the sink's decision, not tarcie's.

The log is never fsynced and never fails a capture. A write that cannot happen
stops at stderr.

## Philosophy

Tarcie does not fail loudly to the user. Capture must feel instant and invisible. Errors are logged internally but the overlay never shows error dialogs or failure states. If a capture fails, the 5-second revert constraint ensures the user is not blocked.

`effectFor` in `src/capture.ts` holds that line. Only a confirmed capture
flashes and hides the overlay. A refusal and a timeout change nothing on
screen: the overlay stays open, holding the text, and says nothing about what
happened.

## What the overlay declines to send

Two gestures produce no capture at all, and both are silent in the same way.

**An empty box.** The hotkey opens the overlay ready for typing, so a reflexive
Enter would otherwise queue an event with no content, deliver it, and archive it
for good. Nothing typed is nothing to capture.

**A second gesture while one is still running.** The box is not cleared until
the flash ends, so a second Enter inside that window sent the same text again
under a fresh `id`. Deduplication downstream is on `id`, so nothing would catch
the pair. One capture runs at a time, and the next gesture is taken as soon as
the one before it is done.

Neither guard can cost a capture. A gesture that is turned away leaves the text
on screen, which is where every unconfirmed capture leaves it anyway.

`capture_note` refuses a note that says nothing of its own, which covers more
ground than the overlay does: an empty box, whitespace, a tag alone, and a
string of tags alike. Section 3 states the rule.

The split is deliberate. The overlay stops the obvious case so that an empty
Enter costs no round trip. The command decides what counts as a note, so the tag
pattern stays in one place rather than in two that can drift apart.

A refusal from the command looks like every other refusal on screen: no flash,
no hide, and the text still in the box. The window that did not go away is the whole signal.

The text on screen is also the only copy anyone can point to when a capture is
unproven, which is the second reason an unconfirmed capture never clears it.

The box is cleared by the capture that took it, and only once that capture is
confirmed. `effectFor` takes whether the capture took the box for that reason,
which is the question that matters. It used to take the kind of capture
instead, and the kind is only a proxy.

Both halves of the rule earn their place. A marker once cleared the box
whatever it held, so text typed and never captured was erased by a click that
had nothing to do with it. A marker that carries the text as its label has
everything to do with it, and leaving that text behind would invite the same
label being sent twice.

A marker over an empty box takes nothing and clears nothing.

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

## Localhost-Only Default

By default, `TARCIE_ALLOW_REMOTE_SINK` is `false`. This means the sink URL must resolve to `127.0.0.1` or `localhost`. Any attempt to configure a remote sink URL without explicitly setting `TARCIE_ALLOW_REMOTE_SINK=true` will be rejected at startup.

This is a safety constraint: Tarcie captures raw, unfiltered user text. Sending it to a remote endpoint without explicit opt-in would be a data leak.

## Configuration Source

All config is read in `sink/config.rs` and assembled into a `SinkConfig` struct at application startup. The config is immutable for the lifetime of the process.

---

# 8. Constraints

Rules 2 to 5 are hardcoded in `constraints.rs`. Rule 1 is a promise about what
the user experiences, so the overlay keeps it: `CAPTURE_TIMEOUT_MS` lives in
`src/capture.ts`. All five are non-negotiable.

## The Five Rules

### 1. Capture Latency: 5-Second Revert

If any capture operation (note or marker) takes longer than 5 seconds, the operation must revert. The user must never be blocked waiting for a capture to complete. This protects the "friction-free" guarantee -- if the queue is broken, the user should not notice.

`runCapture` in `src/capture.ts` races the command against
`CAPTURE_TIMEOUT_MS`. When the budget runs out the overlay stops waiting and
the outcome is `unconfirmed` — not a failure, because the queue may hold the
event after all. The overlay then says nothing and keeps the text, per section
9.

A late reply is ignored. Without the revert, a command that answered a minute
later still confirmed, hid the window, and cleared the box — over whatever the
user had typed since.

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
| `DEFAULT_CONTEXT` | `"General"` | `app_context` when a note carries no `#tag` |
| `MAX_CONTEXT_CHARS` | 64 | Max length of `app_context` field |
| `MAX_TAG_CHARS` | 32 | Max length of extracted `#tag` |
| `MAX_CONTENT_BYTES` | 10,240 (10 KB) | Max size of `content` field |
| `DEFAULT_FLUSH_INTERVAL_SECS` | 300 | Background flush timer |
| `MIN_FLUSH_INTERVAL_SECS` | 1 | Floor under the flush interval |
| `DEFAULT_BATCH_MAX` | 200 | Events per HTTP POST |
| `DEFAULT_QUEUE_MAX_EVENTS` | 10,000 | Queue cap before rotation |
| `HOTKEY` | `"Ctrl+Alt+T"` | The capture hotkey, parsed into the registered binding |
| `HOTKEY_DEBOUNCE_MS` | 500 | Minimum interval between hotkey activations |
| `SHUTDOWN_FLUSH_SECS` | 5 | How long a close waits for the final flush |
| `SINK_REQUEST_TIMEOUT_SECS` | 30 | How long one POST to the sink may take |
| `MAX_LOG_BYTES` | 1,048,576 (1 MiB) | Log size before rotation, per file |
| `MAX_LOG_LINE_CHARS` | 2,048 | Max length of one log line |

None of these is configurable. The environment variables in section 7 are the
whole configuration surface.

`SINK_REQUEST_TIMEOUT_SECS` and `SHUTDOWN_FLUSH_SECS` are both deadlines, and
they answer different questions. The first bounds one request, so a sink that
stops answering cannot end delivery for the session. The second bounds the
final flush, so a slow sink cannot hold the window open on the way out.

---

# 10. Testing

## Current State

Tarcie has a Rust unit test suite and a frontend unit test suite.

The Rust tests live beside the code they cover, in `#[cfg(test)]` modules in
`queue/jsonl.rs`, `ipc/commands.rs`, `sink/config.rs`, `sink/client.rs`,
`flusher.rs`, `util/device.rs`, `util/log.rs`, and `main.rs`.

The frontend tests run under Vitest and live beside the code they cover:

- `src/capture.test.ts` covers `src/capture.ts`, which holds the capture flow
  apart from the DOM and from Tauri — the five-second revert of constraint 1,
  and the rule that the box is cleared by the capture that took it. It runs in the
  `node` environment, which is also a check that `capture.ts` needs no DOM.
- `src/overlay.test.ts` covers `src/overlay.ts`, the wiring from a keyboard, a
  button, and a window to those decisions. It declares
  `// @vitest-environment jsdom` at the top of the file.

The suite covers the seven priority areas:

| Area | Module | Proves |
|------|--------|--------|
| Append and read round-trip | `queue/jsonl.rs` | An appended event reads back with its fields intact, and order holds |
| Tolerant read | `queue/jsonl.rs` | A malformed, truncated, or blank line is skipped; valid events still return |
| Cap rotation | `queue/jsonl.rs` | An append at the cap rotates the file first, and every capped event still reaches a claim |
| Content clamping | `ipc/commands.rs` | Oversized content is clamped, not rejected, and stays valid UTF-8 |
| Tag extraction | `ipc/commands.rs` | `#tag` becomes the context; absent tags fall back to the default |
| Sink URL validation | `sink/config.rs` | A remote sink is refused unless the operator opts in |
| FlushResult variants | `flusher.rs` | `Empty`, `Success`, and `Deferred` each occur, and a deferral keeps every event |

The suite also covers these areas:

| Area | Module | Proves |
|------|--------|--------|
| The command layer | `ipc/commands.rs` | A capture passes through clamp, tag, build, and append, and the queue holds the event the user meant |
| Device identity | `util/device.rs` | The ID is minted once, read back on every run after, and replaced when the file is damaged |
| Name reuse after a crash | `queue/jsonl.rs` | A name an earlier run left behind is never taken over, and an exhausted search fails instead of overwriting |
| The hotkey binding | `main.rs` | The documented `HOTKEY` string parses, and it names the combination that gets registered |
| The log | `util/log.rs` | A report reaches the file stamped, the file rotates at its ceiling and keeps one previous, an over-long line is shortened, and a log that cannot be written does not take the caller down |
| The request bound | `sink/client.rs` | A sink that never answers is given up on at the bound, and a sink that answers inside the bound is not cut off |
| A sink that stops answering | `flusher.rs` | The production path carries a deadline, so a flush over a silent sink ends instead of running on |
| The deferral reason | `flusher.rs` | A deferral names the cause and not only the attempt |
| Nothing worth sending | `ipc/commands.rs` | A note that says nothing of its own never reaches the queue, whether it is empty, whitespace, a tag alone, or a string of tags, and a tagged observation still does |
| Marker labels | `ipc/commands.rs` | A label that is only a tag names the moment, and a label with text beside it splits into the tag and the rest |
| One capture per gesture | `src/overlay.ts` | An empty box sends nothing, a repeated Enter sends one capture, a refused note keeps its text on screen, and the next note is still taken |
| Durable placement | `queue/jsonl.rs` | A placement moves the file and neither directory sync errors, and a placement that cannot happen leaves the batch where it was |
| The capture revert | `src/capture.ts` | A capture that outlives its budget reverts, a slow one inside the budget still counts, and a late reply is ignored |
| Overlay honesty | `src/capture.ts` | Only a confirmed capture flashes and hides the overlay, and the box is cleared only by a confirmed capture that took it |
| The overlay wiring | `src/overlay.ts` | Enter sends what was on screen, Escape puts the overlay away without capturing, the marker button captures with the box as its label or with none when the box is empty, and the overlay arrives focused |
| Text the user has not lost | `src/overlay.ts` | A refusal and a timeout both leave the box and the window alone, and a reply arriving after the revert never takes text typed since |

Each command needs a Tauri `State`, which a test cannot supply. Each one
therefore delegates to a function over a plain `&AppState` — `capture_note_into`,
`capture_marker_into`, `flush_now_on` — and the test drives that function. The
command itself adds only the state extraction.

`load_or_create_device_id` resolves the identity file from the platform data
dir. `load_or_create_device_id_at` takes the path directly, so a test writes to
a temporary directory instead of the real user profile. `JsonlQueue::new_in`
gives the queue the same seam.

`free_path` takes the name generator as an argument. A test can therefore offer
a name that is already on disk, which is what a restarted sequence does, and
hold the guard to leaving that file alone.

`runCapture` takes the send and the budget as arguments, so a test drives both
with a fake clock and never waits five real seconds.

`wireOverlay` takes its ports as an argument: the four elements, the invoke
call, and the window's hide. `src/main.ts` supplies the real ones; a test
supplies a document it built and calls it can drive. `src/main.ts` is then
bootstrap only.

`SinkClient::with_timeout` takes the request bound directly, so a test proves
it in a quarter of a second rather than waiting out `SINK_REQUEST_TIMEOUT_SECS`.
It is `cfg(test)`, because it also accepts no bound at all and production must
not be able to ask for that.

The bound is proven on a real clock, which is deliberate. A paused clock
advances to the nearest deadline as soon as the runtime idles, and waiting on a
real socket idles it. A paused test therefore cannot tell a sink that never
answers from one that answers at once, because both end at the bound. Two
real-clock tests hold it instead: a silent sink is given up on, and a healthy
sink is not cut off. The second is what keeps the first from passing against a
client that refuses everything.

One paused test needs a real exchange to succeed —
`a_partial_multi_batch_delivery_retries_only_what_is_owed`, where the sink must
accept the first batch. It runs through `flusher_unbounded`, with no bound for
the paused clock to jump to. The paused test beside it holds the opposite
ground: with no bound in `SinkClient::new` there is no deadline at all, the
flush never returns, and its guard reports that rather than hanging the suite.

`LogFile::with_ceiling` takes the size ceiling directly, so a test proves the
rotation without writing a megabyte to do it. The tests build a `LogFile` in a
temporary directory and never call `log::init_in`, so the process-wide log stays
closed and nothing in the suite writes to the real user profile.

Every test asserts intended behavior. No test currently pins a known deviation.

One stood briefly: a marker cleared a note the user typed and never captured,
because a confirmed capture of any kind cleared the box. The test that recorded
the deviation asserts the fix, and now states the rule that replaced it — the
box is cleared by the capture that took it, and only once confirmed.

A test that must record behavior differing from the documented intent is marked
with a `KNOWN DEVIATION` comment that states the deviation. A fix must change
that test in the same commit.

## Prerequisites

**Node 22.22.2 or later**, as `package.json` declares. jsdom 30 needs it: on
Node 20 its undici dependency reaches for `webidl` internals that are not there
yet, and the overlay suite dies on import rather than failing a test. CI pins
the same version.

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

## Continuous Integration

`.github/workflows/ci.yml` runs on every pull request and on a push to `main`.
It installs the system libraries, runs `npm ci` and `npm run build`, runs
`npm run check` and `npm test`, runs `cargo test`, and then runs
`bash doc/system/BUILD.sh`.

`npm run check` is `tsc --noEmit`. Vite builds with esbuild, which strips the
types without checking them, so nothing enforced the strict settings in
`tsconfig.json` before this step existed.

The final step runs `git diff --exit-code doc/TARSYSTEM.md`. A change under
`doc/system/` that ships without a rebuild fails the job.

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
cargo test        # the Rust suite
```

```bash
npm test          # the frontend suite
npm run check     # tsc --noEmit
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

These areas have no tests:

1. **The mid-flush capture window at the flusher level.** The window itself is
   covered in `queue::jsonl`, where an append can be placed between the claim
   and the completion. A flusher test cannot reach inside `flush_with_retry`
   to do that, so no end-to-end version exists.
2. **A crash between a partial delivery and its archive.** The duplicate this
   produces is documented, not tested; it needs process-level fault injection.
3. **That every stamped path goes through `free_path`.** The guard itself has
   tests. That each of the four callers uses it — claim, defer, archive, and
   cap rotation — is verified by reading. Forcing a collision through a caller
   needs control of the clock and the sequence, which no seam offers. The same
   reading covers `rename_durably`, which the same four callers use.
4. **That a placement survives a power loss.** `rename_durably` and the append
   that creates the queue file both sync the directory, and the tests prove the
   syncs run and cost nothing in behaviour. Whether the entry is on the platter
   afterwards is a property of the disk, and proving it needs power-loss
   injection rather than a unit test.
5. **Hotkey registration, the window toggle, and the shutdown flush.** All
   three run on the event loop of a real window, so they need a running
   desktop session. The hotkey *string* is covered: a test proves `HOTKEY`
   parses and names the combination the code registers. Whether the operating
   system then grants that combination is not covered.
6. **The bootstrap in `src/main.ts`.** It finds the four elements and hands
   `wireOverlay` the real Tauri calls, and does nothing else. Reaching it needs
   a running webview, because `@tauri-apps/api` only resolves inside one. The
   wiring it hands over is covered in `src/overlay.test.ts`.

---

# 11. Handover

## Implementation Status

**Tarcie v1.0.0** -- stable, feature-complete for v1 scope.

All modules implemented: IPC commands, JSONL queue, HTTP sink client, background flusher, data model, constraints, state management, platform paths, global hotkey.

Delivery uses a **claim**: the flush renames `queue.jsonl` into `sending/`
before it posts anything, so an event captured during a flush cannot be
archived as sent. Section 5 describes the lifecycle and section 6 the loop.
Read those two before changing anything in `flusher.rs` or `queue/jsonl.rs`.

The repository has 96 Rust unit tests and 29 frontend unit tests, and a CI
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
- **The queue grows while the sink is unreachable.** Cap rotation bounds the
  size of one file, not the number of files, and a claim reads every one of
  them into memory. A sink that stays down therefore costs disk and memory.
  The contract prefers that to discarding a capture.
- **The sent archive is never pruned.** Nothing in tarcie deletes a file, so
  every delivered event stays on the disk under `queue/sent/` for the life of
  the installation. That is unbounded disk, and a complete plain-text copy of
  every capture, on a tool that has no encryption at rest. Section 5 records
  the decision this leaves open.
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
