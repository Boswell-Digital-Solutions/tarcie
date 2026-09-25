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
| `CLAIM_MAX_EVENTS` | 5,000 | How many events one claim takes into memory |
| `MAX_SCHEDULED_ROUNDS` | 64 | Bounded rounds one scheduled delivery runs |
| `HOTKEY` | `"Ctrl+Alt+T"` | The capture hotkey, parsed into the registered binding |
| `HOTKEY_DEBOUNCE_MS` | 500 | Minimum interval between hotkey activations |
| `SHUTDOWN_FLUSH_SECS` | 5 | How long a close waits for the final flush |
| `SINK_REQUEST_TIMEOUT_SECS` | 30 | How long one POST to the sink may take |
| `SENT_RETENTION_DAYS` | 90 | How long a delivered batch stays in the archive |
| `SENT_MAX_BYTES` | 268,435,456 (256 MiB) | Ceiling on the whole archive |
| `MAX_LOG_BYTES` | 1,048,576 (1 MiB) | Log size before rotation, per file |
| `MAX_LOG_LINE_CHARS` | 2,048 | Max length of one log line |

None of these is configurable. The environment variables in section 7 are the
whole configuration surface.

`SINK_REQUEST_TIMEOUT_SECS` and `SHUTDOWN_FLUSH_SECS` are both deadlines, and
they answer different questions. The first bounds one request, so a sink that
stops answering cannot end delivery for the session. The second bounds the
final flush, so a slow sink cannot hold the window open on the way out.
