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
