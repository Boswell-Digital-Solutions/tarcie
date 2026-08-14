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
`queue.jsonl` is renamed to:

```
queue.cap.{STAMP}.jsonl
```

A fresh `queue.jsonl` is created for new events. This prevents unbounded file
growth if the sink is unreachable for an extended period.

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

## Capacity

| Parameter | Default |
|-----------|---------|
| Max events before cap rotation | 10,000 |
| Max content per event | 10 KB |
| Max batch size per flush | 200 events |
