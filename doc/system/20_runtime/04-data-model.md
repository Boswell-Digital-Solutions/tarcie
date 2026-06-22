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
