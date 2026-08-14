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
2. Refuse the note if nothing is left, before anything is written
3. Extract the first `#tag` as `app_context`, clamped to `MAX_TAG_CHARS` (32)
4. Build a `TarcieEvent` with:
   - Fresh UUID
   - Device ID from state
   - UTC timestamp + monotonic offset
   - `EventType::Note`
5. Append the event to the JSONL queue (fsync-durable)

**Returns:** `Ok(())`. There is no success payload.

**Refusals and errors:** A note with nothing in it is refused, and nothing is
written. A queue write failure returns the stringified error.

Step 2 is a guard on the queue rather than on the user. An empty event is as
durable as any other — queued, delivered, and archived for good — so the
cheapest place to stop one is before it is written. The overlay refuses one
before it reaches here; this is the boundary that holds when something else
asks.

A tag on its own passes the check, because the text still carries the tag at
that point. `#bug` is therefore a note with an empty `content` and an
`app_context` of `bug`. That is deliberate: a marker carries no tag, so a
tag-only note is the only way to mark a moment with a label.

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

1. Clamp `reason` to `MAX_CONTENT_BYTES` if one was given
2. Build a `TarcieEvent` with:
   - Fresh UUID
   - Device ID from state
   - UTC timestamp + monotonic offset
   - `EventType::Marker { reason }`
   - An empty `content` and the default `app_context`
3. Append the event to the JSONL queue (fsync-durable)

**Returns:** `Ok(())`. There is no success payload.

**Errors:** Returns the stringified error on a queue write failure.

A marker carries no text by design, so the empty check that guards
`capture_note` does not apply. The gesture is the observation.

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
