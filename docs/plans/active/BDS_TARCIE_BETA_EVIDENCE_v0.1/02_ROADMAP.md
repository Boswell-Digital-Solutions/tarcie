# Tarcie Beta Evidence Roadmap

**Plan:** `BDS-TARCIE-BETA-EVIDENCE-v0.1`  
**Roadmap posture:** gates, not dates; progression requires receipts and explicit decisions

## Phase 0 — Source lock and baseline

**Outcome:** a verified current-state evidence ledger.

- Pin repository commits and protocol revisions.
- Reconcile actual Tarcie capture paths, queue behavior, and tests.
- Locate or define the Forge_Command intake owner.
- Locate the exact DataForge Local artifact/receipt persistence boundary.
- Record note latency, queue recovery, duplicate behavior, disk usage, and current security posture.
- Confirm Linux as the first qualification platform; treat other platforms as unqualified.

**Exit:** GATE-00 receipt and Board Review 1 decision.

## Phase 1 — Contract and fixture spine

**Outcome:** no live capture dependency; all evidence contracts work against fixtures.

- Define `BetaSession.v1`, `BetaObservation.v1`, `BetaArtifact.v1`, `BetaEvidenceSubmission.v1`, and `BetaEvidenceAcceptanceReceipt.v1`.
- Publish positive, negative, conflict, partial-acceptance, and replay fixtures.
- Define deterministic canonicalization and hashing.
- Build Forge_Command validator and receipt generator against isolated fixtures.

**Exit:** schema validation, negative tests, canonical hash vectors, and receipt contract proof.

## Phase 2 — Local capture proving slice

**Outcome:** Tarcie reliably captures notes and explicit screenshots locally.

- Add manual beta-session start/end.
- Add active-window screenshot capture with explicit permission.
- Store screenshots separately from JSONL.
- Add artifact manifest, SHA-256 verification, spool cap, restart recovery, and visible local/queued state.
- Preserve fast hotkey capture and no-AI boundary.

**Exit:** GATE-01 receipt.

## Phase 3 — Localhost handoff

**Outcome:** receipted Tarcie-to-Forge_Command delivery.

- Add localhost-only intake.
- Implement idempotent resumable submission.
- Verify artifact hashes before acceptance.
- Return per-item acceptance/rejection and a hash-bound receipt.
- Persist accepted evidence through the bounded DataForge Local interface.

**Exit:** GATE-02 receipt and fault-injection report.

## Phase 4 — Beta Evidence Inbox

**Outcome:** truthful, read-only operator review.

- Add session list, timeline, screenshot preview, note detail, hashes, source/build identity, and receipt status.
- Separate live, fixture, replay, deferred, rejected, unavailable, and partial states.
- Add bounded operator annotations without altering source evidence.
- Add accessible keyboard navigation and non-color state redundancy.

**Exit:** GATE-03 acceptance and comprehension tests.

## Phase 5 — Privacy and operational qualification

**Outcome:** evidence capture is safe enough for a limited internal beta.

- Denylist/redaction controls.
- Retention and cleanup receipts.
- Encryption-at-rest decision and implementation if required.
- Disk-full, permission-denial, process-kill, receiver-down, duplicate, corrupt spool, and hash-conflict tests.
- Performance and accessibility qualification.
- RED Board abuse-case review.

**Exit:** GATE-04 receipt.

## Phase 6 — Limited beta and Board Review 2

**Outcome:** shadow-qualified workflow and explicit release decision.

- Run representative internal beta sessions across approved applications.
- Measure capture completion, evidence loss, duplicate detection, receipt completeness, privacy incidents, operator comprehension, and recovery.
- Reconcile findings without automatically changing baselines or policy.
- Produce Board Review 2 packet with AUTHORIZE, REWORK, or REJECT recommendation.

**Exit:** explicit human authorization for any broader beta.

## Deferred roadmap

These require separate plan revisions after the local proving slice:

- Windows or macOS qualification.
- Remote/cloud synchronization.
- Team/multi-user sessions.
- Video or audio capture.
- OCR or AI-assisted summarization.
- Automatic GitHub issue proposals.
- Integration with YellowJacket status visualization.
- Agent-assisted reproduction or automated testing.

None is inherited as authority from this roadmap.

