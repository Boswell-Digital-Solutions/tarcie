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

The sent directory is bounded by two rules, applied together:

| Bound | Value | Drops |
|---|---|---|
| `SENT_RETENTION_DAYS` | 90 days | A batch stamped before the cutoff |
| `SENT_MAX_BYTES` | 256 MiB | The oldest batches, until the total fits |

Both are sized for an ordinary desktop. A typical note is about 310 bytes on
the line, so a hundred captures a day costs roughly 11 MB a year and never
approaches the ceiling. The ceiling is for content that runs to
`MAX_CONTENT_BYTES`, where the same hundred a day would reach about 370 MB a
year.

The archive used to keep every event ever captured, for the life of the
installation. That was unbounded disk, and a complete plain-text copy of
everything the user had captured on a tool with no encryption at rest.

### What the archive is for

Nothing in tarcie reads it. The tool is write-only, so the archive cannot be
searched, resent, or displayed, and recovering anything from it means a person
opening files by hand.

It is therefore forensic, not a safety net. The durability contract — a capture
survives an unreachable sink — is kept by `queue.jsonl` and `sending/`, which
both sit before delivery. The archive is entirely after it.

### When the bounds are applied

- **Whenever a batch is archived.** Archiving is the only thing that grows the
  archive, so it is where the archive is bounded. A claim that placed nothing
  skips the pass: it runs under the lock `append` waits on, and reading the
  whole directory every flush cycle would put that in the capture path for no
  reason.
- **Once at startup.** A run that delivers nothing never grows the archive and
  would never revisit it, so the retention period would go unkept on exactly
  the installations holding the most forgotten captures.

### What is never deleted

A file whose name does not carry a stamp this can read. Nothing but the archive
step writes to that directory, so a name of another shape arrived by hand, and
deleting what it cannot date is not a capture tool's business.

### The record

Deleting a capture is the one thing tarcie does that a user cannot see and
cannot undo, so it is never silent. One log line carries the count, the byte
total, and the span of stamps removed. What the events said never appears,
which is the rule the rest of the log already keeps.

## Capacity

| Parameter | Default |
|-----------|---------|
| Max events before cap rotation | 10,000 |
| Max content per event | 10 KB |
| Max batch size per flush | 200 events |
