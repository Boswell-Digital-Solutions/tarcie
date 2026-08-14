# GATE-00 Session Brief — BDS-TARCIE-BETA-EVIDENCE-v0.1

**Prepared:** 2026-08-14
**Status:** preparation input. Not a GATE-00 receipt, not a packet file, not a
Board Review 1 answer.

## What this is

Reconnaissance for the source lock, gathered so a GATE-00 session starts from
verified facts instead of re-deriving them.

## What this is not

This grants nothing. It does not change the plan set's lifecycle, authority
status, manifest, or registry record. The packet's `supersession_rule` requires
a new reviewed revision for any semantic change, and Board Review 1 is open, so
nothing here edits `04_SOURCE_BASELINE.md` or any other reviewed document.

Where this brief and the packet disagree, **the packet is the document under
review** and this is a claim about the world that the board may accept or reject.

---

## 1. Pinned heads, verified 2026-08-14

| Repository | Head | Date | Against the packet |
|---|---|---|---|
| `tarcie` | `062e7935d46c8d8dc75c4193afa9d53f77dc0820` | 2026-08-14 | **moved** — packet pins `309a231b…` (2026-07-29) |
| `Forge_Command` | `fbcf51e1575e…` | 2026-08-11 | **unchanged** — matches the packet pin exactly |
| `DataForge Local` | `6c07d13d31a6…` | 2026-08-13 | not pinned by the packet |
| `forge_contract_core` | `f7de57457ab5…` | 2026-08-09 | not pinned by the packet |
| `DataForge` (cloud) | `63c213f4b884…` | 2026-08-10 | not pinned by the packet |

Only tarcie's baseline has drifted. Sibling rows are local checkout heads; a
GATE-00 session should re-pin from `origin/main` before locking.

---

## 2. What changed in tarcie since the packet's pin

The packet lists among tarcie's known gaps: *"no screenshot capture, session
identity, artifact contract, receiver acknowledgment receipt, readback,
encryption-at-rest decision, or meaningful automated test suite."*

**Every item still holds except the last.** Since `309a231`:

- 85 Rust unit tests and 22 frontend unit tests
- CI gates both suites, a TypeScript typecheck, the document build, and a
  stale-artifact check on every pull request
- an operational log under `logs_dir`, bounded and free of capture content
- corrections for defects that could cost a capture: a flush that could archive
  an unsent event, a file name a restarted run could take over, a queue
  discarded rather than delivered at its cap, a capture budget documented but
  never implemented, and two paths that cleared text the user never captured

No new capability was added. Tarcie is still note and marker capture, a JSONL
queue, and a generic HTTP sink.

---

## 3. Phase 0 checklist

| Item | Status |
|---|---|
| Pin repository commits and protocol revisions | **Ready** — section 1; protocol revisions still to confirm |
| Reconcile actual capture paths, queue behavior, and tests | **Answered** — section 4 |
| Locate or define the Forge_Command intake owner | **Answered: does not exist** — section 5 |
| Locate the DataForge Local artifact/receipt boundary | **Partial** — section 6 |
| Record queue recovery, duplicate behavior, disk usage, security posture | **Answered** — section 4 |
| Record note latency | **Not answerable here** — needs a live desktop session |
| Confirm Linux as the first qualification platform | **Consistent with the repo** — section 7 |

---

## 4. Tarcie source facts, with evidence

### Can the queue resend an already delivered batch?

The packet lists this as requiring verification. **In normal operation, no.**

`JsonlQueue::defer(claim, delivered)` writes back only `claim.events[delivered..]`,
so a batch the sink accepted is never offered again. Proven by
`flusher::tests::a_partial_multi_batch_delivery_retries_only_what_is_owed`.

**Two duplicate paths remain, both deliberate:**

1. The remainder is written back *before* the originals are archived. A crash
   between those two steps re-offers the remainder. The contract prefers a
   duplicate to a loss. Documented in section 11; **untested** — it needs
   process-level fault injection (section 10, item 2).
2. The sink is never asked whether it already holds an event, so any retry
   after an unacknowledged success resends. Deduplication belongs downstream,
   on `id`.

Consequence for the plan: `BetaEvidenceAcceptanceReceipt.v1` must be idempotent
on `id`. That is an intake requirement, not a tarcie fix.

### Queue recovery

A claim picks up batches an interrupted run left in `sending/`, oldest first —
`queue::jsonl::tests::a_claim_keeps_what_an_earlier_run_left_in_sending`.

File names are never reused across a restart. `free_path` takes a fresh stamp
rather than renaming onto an existing file, and fails the placement rather than
overwriting.

### Disk usage

- **Queue:** cap rotation bounds one *file* (10,000 events by default), not the
  number of files. A sink that stays unreachable grows `sending/`, and a claim
  reads every pending file into memory. Recorded as a known limitation.
- **Log:** bounded at two files of `MAX_LOG_BYTES` (1 MiB each). One line is
  capped at `MAX_LOG_LINE_CHARS` (2048).
- **Screenshots:** none. Phase 2 introduces the first unbounded artifact class,
  and the plan's spool cap is the control.

### Security posture

- Localhost-only sink by default; a remote sink needs
  `TARCIE_ALLOW_REMOTE_SINK` set explicitly. Loopback is matched on the parsed
  host, not a string prefix.
- The auth token is never logged. `SinkConfig` deliberately does not derive
  `Debug`, and the sink URL is reported through `url_without_credentials`.
- The operational log holds no capture content by invariant.
- **No encryption at rest.** Queue, log, and device ID sit in the platform data
  directory as plain files. The packet already lists this as an open decision.
- **`tauri.conf.json` sets `"csp": null`**, disabling the webview Content
  Security Policy. No injection vector exists today — the overlay renders no
  user content as HTML — but Phase 2 adds screenshots and Phase 4 adds a review
  surface, and both change that. Worth a decision before Phase 2.

### Note latency

**Not measured.** The five-second revert is a ceiling on how long the overlay
waits before giving up, not a measurement of capture latency. Phase 0's latency
record and GATE-01's "capture p95 within the approved threshold" both need a
live desktop session. No threshold has been set yet.

---

## 5. Forge_Command intake — confirmed absent

The packet records "no verified `/ingest/tarcie` receiver or Tarcie-specific
Beta Evidence Inbox was found." **Still true at `fbcf51e1575e`.**

- The only ingest route is `POST /telemetry/ingest`
  (`api/src/routes/telemetry.rs`). It is telemetry, not evidence.
- Evidence routes exist but are cloud-diagnostic and proposal shaped
  (`api/src/routes/cloud/`), not an intake.
- The API binds `127.0.0.1:8004` by default (`api/src/config.rs`).

**Port mismatch worth recording:** tarcie's default sink is
`http://127.0.0.1:8080/ingest/tarcie`. Nothing serves that. The default points
at neither Forge_Command nor any existing receiver, so today's default
configuration cannot deliver. Phase 3 defines the intake; Phase 0 should record
that the current default is aspirational.

---

## 6. DataForge Local persistence boundary — partial

Head `6c07d13d31a6`, eight routers mounted in `app/main.py`.

Relevant existing shapes:

- `app/api/lineage_router.py` issues `LineageWriteReceipt.v1` with a
  `receipt_id`, and handles `LineageIngestEnvelope.v1`.
- `app/api/proving_slice_queue_router.py` reads a `ps_local_artifacts` table in
  the runtime-promotion schema, with artifact detail for operator inspection.

So an artifact table and a write-receipt pattern both exist, but they are
proving-slice shaped rather than beta-evidence shaped. A GATE-00 session should
decide whether `BetaArtifact.v1` reuses `ps_local_artifacts` or needs its own
boundary — that is a design question the packet leaves open.

---

## 7. Contract families — a Phase 1 prerequisite the plan does not state

`forge_contract_core` at `f7de57457ab5` admits roughly seventy families in
`ADMITTED_FAMILIES` (`forge_contract_core/enums.py`). **None is a `beta_*`
family.** There is no `beta_session`, `beta_observation`, `beta_artifact`,
`beta_evidence_submission`, or acceptance-receipt family.

That repo's rule: *"Adding a family requires an RFC."* Phase 1 proposes five new
contract families, so **Phase 1 carries an unstated RFC prerequisite in an
upstream authority repo.** Board Review 1 should decide whether those families
are admitted to `forge_contract_core` or held local to tarcie and Forge_Command
until the proving slice earns them.

Reusable prior art: the `telemetry_emit_receipt`, `promotion_receipt`, and
`bugcheck_*` receipt families, and the fixture layout under `fixtures/`
(`invalid`, `duplicate`, `adversarial`, `restricted`, `read_model`) which
matches Phase 1's requested positive/negative/conflict/replay set.

---

## 8. Linux first — consistent with the repo

- CI runs `ubuntu-latest` only.
- `tauri.conf.json` bundles `deb` and `appimage`; no Windows or macOS target.
- The handover chapter records Windows IPC path edge cases as untested.

Nothing contradicts Linux-first qualification.

---

## 9. Reproducing this

```bash
# tarcie gate, from the repo root, on Node 22.22.2 or later
npm ci && npm run build
npm run check && npm test
cargo test --manifest-path src-tauri/Cargo.toml
bash doc/system/BUILD.sh && git diff --exit-code doc/TARSYSTEM.md
```

Expected: 85 Rust tests, 22 frontend tests, `BUILD_OK designation=TAR parts=20`,
and a clean diff.

Cross-repo checks used here were read-only greps over `Forge_Command/api/src`,
`dataforge-Local/app`, and `forge_contract_core/forge_contract_core/enums.py`.
No sibling repository was modified.

---

## 10. Suggested first moves for a GATE-00 session

1. Re-pin all five heads from `origin/main` and record them.
2. Decide the disposition of the packet's stale tarcie baseline: amend under a
   new reviewed revision, or annotate at the gate. Its supersession rule points
   at the first.
3. Take the two decisions that change Phase 1's shape: whether the five beta
   families go through a `forge_contract_core` RFC, and whether `BetaArtifact.v1`
   reuses the existing DataForge Local artifact boundary.
4. Set the capture-latency threshold GATE-01 will be measured against. It does
   not exist yet.
5. Decide `"csp": null` before Phase 2 adds screenshots.

Items 3 to 5 are operator decisions. Nothing in this brief settles them.
