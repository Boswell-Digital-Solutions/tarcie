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
