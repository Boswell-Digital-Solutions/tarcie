# tarcie Extended Roadmap

**Document version:** 1.2 (2026-08-14) — reconciled with the working tree

## Current Status

| Phase | Status | Outcome |
| --- | --- | --- |
| Documentation normalization | Complete | Protocol-required baseline surfaces are present |
| Verification hardening | Complete | Rust and frontend suites, both gated in CI alongside a typecheck |
| Module catalog expansion | Pending | Expand exact routes, tables, and runtime contracts from code |

## Phase 0 — Documentation Normalization

**Goal:** establish the required documentation stack and build surfaces.

**Delivered:**

- root `CLAUDE.md`
- modular `doc/system/`
- `doc/system/BUILD.sh`
- `doc/TARSYSTEM.md`
- `scripts/context-bundle.sh`

## Phase 2 — Verification Hardening

**Goal:** align repo-specific testing, QA, and handover documentation with
current implementation reality.

**Delivered:**

- 124 Rust unit tests across `queue/jsonl.rs`, `ipc/commands.rs`,
  `sink/config.rs`, `sink/client.rs`, `flusher.rs`, `util/device.rs`,
  `util/log.rs`, and `main.rs`
- 29 frontend unit tests across `src/capture.ts` and `src/overlay.ts`, under
  Vitest, with jsdom for the overlay
- `.github/workflows/ci.yml`, which builds the frontend, typechecks it, runs
  both suites and the document build on every pull request, and fails on a
  stale `doc/TARSYSTEM.md`
- an operational log under `logs_dir`, bounded and free of capture content,
  because a write-only tool otherwise has no way to report that delivery
  stopped
- corrections for every defect the suite and a close reading exposed. Those
  that could cost a capture: a flush that could archive an unsent event, a file
  name a restarted run could take over, a queue that was discarded rather than
  delivered on reaching its cap, a capture budget that was documented and never
  implemented, and two paths that cleared text the user had never captured
- a bound on every request to the sink. A sink that accepted the connection and
  then stopped answering held the flush open, and the background flusher is one
  task, so that flush ended delivery for the session without a word. The
  deferral reason now also names the cause rather than the attempt, because it
  is what the log carries

Section 10 of the system document records what the tests cover and what they do
not. The uncovered set is the honest starting point for more work here.

## Phase 1 — Exact Surface Expansion

**Goal:** replace baseline placeholders with exact module, API, schema, and
environment documentation.

This is the remaining phase. The runtime chapters (sections 4 to 6) and the
error-handling chapter are exact, because the durability, revert, and logging
work rewrote them against the code. Section 7 gained a floors table. The
command reference was corrected against the signatures it documents, and the
product surface chapter is now authored from the window configuration, the
overlay markup, and the gestures the wiring binds.

Three registry-generated scaffolds remain, and each still carries the notice
that says so:

- `doc/system/20_runtime/20-runtime.md`
- `doc/system/30_dependencies/40-integrations.md`
- `doc/system/50_operations/50-operations.md`

The bootstrap appendices duplicate the overview and architecture chapters
rather than scaffolding, so they need reconciling rather than authoring.

## Governed forward plan

`BDS-TARCIE-BETA-EVIDENCE-v0.1`, in `docs/plans/active/`, proposes what tarcie
becomes next: a bounded beta-testing observation and evidence assistant that
packages notes and screenshots as verifiable evidence for Forge_Command review.

That plan set is `proposed` and documentation-only. Board Review 1 is open and
its next gate is GATE-00, a source lock. It grants no implementation authority,
and neither does this roadmap. This roadmap covers only the repository's own
documentation and verification maturity.

**Its recorded source baseline predates this tree.** The plan set pins
`309a231b0ce9ee7c1de88136d1d07356ffdfe93d`, dated 2026-07-29, and lists "no
meaningful automated test suite" among tarcie's gaps. Both suites and the CI
gate landed after that commit, and the plan's open question about whether the
queue can resend an already delivered batch is now answerable from the code and
its tests.

GATE-00 asks for exactly that reconciliation. The plan set is under review and
its supersession rule requires a new reviewed revision for any semantic change,
so this roadmap records the drift rather than editing the packet.
