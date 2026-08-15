# Retention Gap — the sent archive is outside §8

**Prepared:** 2026-08-14
**Status:** preparation input for Board Review 1 and GATE-00. Not a receipt, not
a packet file, not a decision.

## What this is

A gap between one prerequisite in the canonical plan and one behaviour in the
current tarcie tree. The prerequisite is sound. It does not reach the store that
already exists.

## What this is not

This grants nothing and decides nothing. The plan set is `proposed` and
`documentation-only`, Board Review 1 is open, and the supersession rule in
`FOLDER_METADATA.json` requires a new reviewed revision for any semantic change.
So this edits none of the reviewed documents, and it is deliberately absent from
`00_README_PLAN_INDEX.md`. It sits beside `00_SESSION_BRIEF.md`, which is the
layout the plan set already uses for gate preparation.

Where this and the packet disagree, the packet is the document under review.

---

## 1. The prerequisite

`01_CANONICAL_PLAN.md` §8, *Privacy and security*:

> Local spool content receives a defined retention period, storage cap, cleanup
> receipt, and encryption-at-rest decision before beta qualification.

Four decisions, named correctly, and bound to beta qualification.

## 2. What it covers

The **local spool** — the artifact store Phase 2 introduces when tarcie gains
screenshot capture. Screenshots are the first unbounded artifact class, and §8
puts a control on them before they exist.

That store does not exist yet. Nothing in the current tree writes it.

## 3. What it does not cover

The **sent archive**, `queue/sent/`, which exists in every installation today.

A delivered batch is renamed to `queue.sent.{STAMP}.jsonl` and left there.
Nothing in tarcie deletes a file. Nothing in tarcie reads that directory, save a
test helper. Every event ever captured therefore stays on the disk after
delivery, verbatim, for the life of the installation.

It holds the `content` field as the user typed it, with `device_id`,
`app_context`, and both timestamps beside it.

This behaviour is tarcie v1.0.0. It predates the plan set and does not depend on
it. If Board Review 1 rejects the plan outright, the archive keeps growing.

## 4. Why the four decisions apply unchanged

Each one lands on the archive as squarely as on the spool.

| Decision | The archive today |
|---|---|
| Retention period | None. A batch stays until the disk is wiped |
| Storage cap | None. Cap rotation bounds one file, never the total |
| Cleanup receipt | Nothing to receipt, because nothing is cleaned |
| Encryption at rest | None. Plain files in the platform data directory |

`00_SESSION_BRIEF.md` §4 already records the absence of encryption at rest for
the queue, log, and device ID. The archive is the largest of those surfaces and
the only one that grows without limit.

## 5. The observation that should shape the decision

**The archive is inert.** Tarcie is write-only by design and has no readback
surface, so nothing in the product can restore from it, resend it, or show it.
Recovering anything from it means a person opening files by hand.

So it is not a recovery mechanism. The durability contract — a capture survives
an unreachable sink — is already kept by `queue.jsonl` and `sending/`, both of
which sit before delivery. The archive is entirely after it.

Its value is therefore forensic and manual, and it should be priced as that
rather than as a safety net.

## 6. What Phase 3 changes

Phase 3 introduces the acknowledgment receipt the packet calls for. Today a
2xx is the only evidence a batch arrived, and it is unverified. After Phase 3, an
acknowledged event is provably held downstream.

That is the natural boundary for a retention rule: **retain until acknowledged,
then for a stated grace period.** Before Phase 3 it cannot be stated in those
terms, because there is nothing to be acknowledged by.

## 7. Options, for the decision owner

Not a recommendation to adopt without review. Each carries its cost.

**Retention period**

1. Keep forever — status quo. Zero work. Unbounded disk, unbounded plaintext.
2. Age out after N days. Bounded. A batch the sink silently dropped is
   unrecoverable after N days.
3. Retain until acknowledged, then a grace period. Strongest, and needs Phase 3.

**Storage cap**

1. None — status quo.
2. A byte or file ceiling, oldest evicted first. Bounds the worst case whatever
   the retention period says. It pairs with any of the three above.

**Cleanup receipt**

Any deletion of user content must leave a record. The operational log already
exists, is bounded, and holds no capture content. A line stating the count, the
byte total, and the oldest and newest stamps removed fits it exactly, and states
nothing about what the events said.

**Encryption at rest**

A single decision covering queue, archive, log, and device ID. Deciding it for
the archive alone would leave `queue.jsonl` in plaintext, holding the same text.
`00_SESSION_BRIEF.md` §10 already lists this among the operator decisions.

## 8. A note on sequencing

Options 1 and 2 under retention need no plan authority. They change no contract,
no schema, and no interface. They are repo-local behaviour in `queue/jsonl.rs`,
and could ship before Board Review 1 concludes.

Option 3 depends on Phase 3 and therefore on the plan.

The distinction matters, because the archive grows either way while the review
runs.

## 9. Suggested disposition

1. Record the gap at GATE-00, so §8 is read as covering both stores.
2. Take the retention and cap decisions for the archive independently of the
   plan, since they need no authority the plan grants.
3. Defer the acknowledgment-bound rule to Phase 3, where it belongs.
4. Keep encryption at rest as one decision across all four local surfaces.

## 10. Addendum — three of the four decisions are taken, 2026-08-15

Sections 1 to 9 record the gap while every decision was open. The decision
owner has since taken three, on the standard that tarcie must sit comfortably on
an ordinary desktop computer.

| Decision | Taken | Value |
|---|---|---|
| Retention period | **Yes** — option 2, age out | `SENT_RETENTION_DAYS` = 90 |
| Storage cap | **Yes** — option 2, oldest evicted first | `SENT_MAX_BYTES` = 256 MiB |
| Cleanup receipt | **Yes** — to the existing log | count, bytes, span of stamps |
| Encryption at rest | **Open** | still one decision across four surfaces |

The values are derived rather than chosen from the air. A typical note is about
310 bytes on the line, so a hundred captures a day costs roughly 11 MB a year
and never approaches the ceiling. The ceiling is sized for content that runs to
`MAX_CONTENT_BYTES`, where the same hundred a day would reach about 370 MB a
year. On a desktop with a 256 GB disk the ceiling is a tenth of one percent.

Section 8's sequencing held: these three needed no authority the plan grants,
and they shipped while Board Review 1 remained open. Option 3 — retain until
acknowledged — is untouched and still belongs to Phase 3, where an
acknowledgment first exists to retain against.

The bounds are applied where the archive grows, and once at startup so that an
installation which delivers nothing still keeps the period. A file whose name
carries no readable stamp is never deleted, because nothing but the archive step
writes there and deleting what it cannot date is not a capture tool's business.

This addendum records what was decided. It is not itself the decision, and it
changes no reviewed document.

## 11. Evidence

Verified against tarcie `6e308ff`, 2026-08-14.

```bash
# Nothing deletes a file. The only truncate is the temp file in write_events.
grep -rn "remove_file\|remove_dir\|truncate(true)" src-tauri/src/

# Nothing reads the sent directory outside a test helper.
grep -rn "sent_path\|sent_dir" src-tauri/src/
```

The behaviour is now documented in `doc/system/20_runtime/05-queue-system.md`
under *Retention*, and in the handover chapter's known limitations. Those
sections state the behaviour and name the decision. They do not take it.
