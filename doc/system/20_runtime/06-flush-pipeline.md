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
- **On exhaustion:** Flush returns `Deferred` with reason. Events remain in the queue file untouched

## FlushResult

The flusher returns one of three outcomes:

| Result | Meaning |
|--------|---------|
| `Empty` | Queue had no events to flush |
| `Success(n)` | Successfully flushed `n` events, queue rotated |
| `Deferred(reason)` | Flush failed after retries, events remain in queue |

## Manual Flush

The `flush_now` IPC command triggers an immediate flush cycle outside the timer. It follows the same logic as the background loop but returns the result directly to the caller.

## Graceful Shutdown

On window close, Tarcie attempts a final flush with a **5-second timeout**. If the flush does not complete within 5 seconds, the application exits and events remain safely in the queue file for the next launch.
