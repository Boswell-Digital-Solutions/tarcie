# 6. Flush Pipeline

The flusher is a background task that periodically drains the JSONL queue and posts events to the HTTP sink. Implemented in `flusher.rs`.

## Flush Loop

1. Sleep for `TARCIE_FLUSH_INTERVAL_SECS` (default: 300 seconds)
2. Acquire the flush Mutex
3. **Claim** the queue — `queue.jsonl` moves into `sending/`, and any batch left
   by an interrupted flush is picked up with it
4. If the claim is empty, archive its files and return to step 1
5. Chunk the claimed events into batches of `DEFAULT_BATCH_MAX` (200)
6. POST each batch to the sink endpoint, counting what is accepted
7. On full success: archive the claim to `queue.sent.{STAMP}.jsonl`
8. On failure partway: **defer** — write the undelivered remainder back to
   `sending/`, archive the originals, and return `Deferred`
9. Release lock, return to step 1

Step 3 is what keeps a capture safe. The queue is moved aside before anything
is posted, so an event captured during delivery is not in the claim and cannot
be archived as sent.

Step 8 is what keeps delivery honest. A flush that accepted three batches and
failed on the fourth retries only the fourth; the three the sink already holds
are not offered again.

## Delivery on a Daily Schedule

When `TARCIE_FLUSH_AT` names a local time, the loop above still ticks on the
interval, but a tick delivers only when today's delivery is owed. `schedule.rs`
holds that rule, and it is held against the calendar rather than against
elapsed time:

- the target time has passed today, **and**
- today is not the day already recorded

**A missed night is recoverable, which is the point.** An interval only fires
while the application runs, so a desktop asleep at 02:00 would skip the night
entirely and an elapsed-time rule would never notice. Because the rule asks
what day it is, a machine that wakes at 09:00 delivers then.

The day is compared for difference rather than for being earlier. A clock that
moved backwards would otherwise stop delivery until the date caught up, and the
contract prefers a duplicate to a night that never arrives. A schedule that has
never delivered is owed one immediately, so an installation upgrading into a
schedule does not hold a backlog for a day.

### One delivery, several rounds

A claim takes at most `CLAIM_MAX_EVENTS`, so a day that captured more than one
claim holds needs more than one round to clear. On the interval the next cycle
is minutes away and this never arises. On a schedule the next cycle is a day
away, and a backlog would never catch up.

A scheduled delivery therefore runs bounded rounds until the queue is `Empty`,
a round defers, or `MAX_SCHEDULED_ROUNDS` is reached. Memory stays bounded,
because each round is still one bounded claim.

### The day is recorded only when the queue is clear

A deferral leaves the day unrecorded, so the next tick tries again rather than
waiting out the night on a sink that was briefly unreachable. The marker is
`last_scheduled_flush.txt` in the data directory, holding one local date.

A marker that cannot be read counts as no marker, which means delivering again.
That costs a duplicate, which the sink deduplicates on `id`; erring the other
way costs a night.

**A capture can now sit undelivered for up to a day.** The queue is what makes
that safe, and section 5 is where to look: the appends are durable, placements
survive a power loss, and the files are closed to other accounts.

## Batch Payload

Each HTTP POST sends:

```json
{
  "source": "tarcie",
  "events": [ <up to 200 TarcieEvent objects> ]
}
```

## Retry Strategy

- **Max retries:** 3 attempts per batch
- **Backoff:** Exponential -- `2^retry` seconds (2s, 4s, 8s)
- **Per request:** `SINK_REQUEST_TIMEOUT_SECS` (30s) bounds one POST
- **On exhaustion:** Flush returns `Deferred` with reason. Events remain in the queue file untouched

The per-request bound is what ends a wait on a sink that stops answering. A
refused connection fails at once and reports itself. A sink that accepts the
connection and then goes quiet holds a healthy connection open and reports
nothing, and `reqwest` applies no time bound of its own.

The bound matters because the background flusher is a single task. A flush that
does not return takes every later flush with it, for the rest of the session.
Captures still reach the queue, nothing leaves it, and the log stays silent,
because a deferral is only logged once a flush ends.

Four attempts and the backoff between them come to 134 seconds. That is inside
the 300-second default flush interval, so a bounded flush finishes before the
next one is due.

## FlushResult

The flusher returns one of three outcomes:

| Result | Meaning |
|--------|---------|
| `Empty` | Queue had no events to flush |
| `Success(n)` | Successfully flushed `n` events, queue rotated |
| `Deferred(reason)` | Flush failed after retries, events remain in queue |

## Manual Flush

The `flush_now` IPC command triggers an immediate flush cycle outside the timer. It follows the same logic as the background loop but returns the result directly to the caller.

## Reporting

The background loop logs a `Deferred` result with its reason. A deferral is the
queue keeping its promise rather than a fault, but it is also the only word
anyone gets that captures are not arriving: tarcie has no readback surface, so
an unreported deferral leaves a sink that has been refusing for days looking
like a sink with nothing to do.

The reason carries the whole cause chain. `anyhow` prints only the outermost
context in its plain `Display`, and every failure inside `post_json` carries the
context `POST to sink`. A refused connection, a timeout, and a name that does
not resolve therefore reported the same four words, which name the attempt and
never the cause. `MAX_LOG_LINE_CHARS` still bounds the line.

`Empty` and `Success` are not logged. A flush that worked has nothing to say.

## Graceful Shutdown

On window close, Tarcie attempts a final flush with a **5-second timeout**. If the flush does not complete within 5 seconds, the application exits and events remain safely in the queue file for the next launch.
