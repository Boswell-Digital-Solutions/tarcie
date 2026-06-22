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
