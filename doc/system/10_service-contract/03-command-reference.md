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
