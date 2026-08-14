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
for good. Nothing typed is nothing to capture. A tag on its own still counts,
because a marker carries no tag and a tag-only note is the only way to mark a
moment with a label.

**A second gesture while one is still running.** The box is not cleared until
the flash ends, so a second Enter inside that window sent the same text again
under a fresh `id`. Deduplication downstream is on `id`, so nothing would catch
the pair. One capture runs at a time, and the next gesture is taken as soon as
the one before it is done.

Neither guard can cost a capture. A gesture that is turned away leaves the text
on screen, which is where every unconfirmed capture leaves it anyway.

`capture_note` refuses an empty note as well. The overlay stops one first, and
the command is the boundary that holds when something else asks. The window that did not go away is the whole signal.

The text on screen is also the only copy anyone can point to when a capture is
unproven, which is the second reason an unconfirmed capture never clears it.

Clearing the box belongs to the note capture alone. `effectFor` takes the
`CaptureKind` for that reason. A marker is a separate gesture that happens to
share the overlay, and it used to take the box with it — so text typed and
never captured was erased by a click that had nothing to do with it.
