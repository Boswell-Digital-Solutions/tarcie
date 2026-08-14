# BDS-TARCIE-BETA-EVIDENCE-v0.1 — Canonical Plan

## 1. Executive decision

Proceed with Tarcie as the operator-facing beta-testing field assistant, subject to Board Review 1 and staged proof.

The product boundary is:

> Tarcie captures beta-testing observations and artifacts. Forge_Command validates, presents, and governs them. DataForge Local stores accepted evidence. SMITH governs any later mutation or promotion path.

The first implementation candidate must remain local, manual, observation-only, and reversible. No AI, automatic defect classification, issue creation, repair, or promotion is included in the first proving slice.

## 2. Problem statement

During beta testing, an operator needs to record what happened without breaking concentration. A useful system must accept a note or screenshot in seconds, bind it to the correct beta session and build, preserve it through failures, and make it reviewable later.

Tarcie already supplies fast hotkey capture, notes, markers, a durable JSONL queue, and generic HTTP delivery. It lacks screenshot capture, test-session identity, artifact hashing, per-event acknowledgments, evidence receipts, and a verified Forge_Command receiver.

Forge_Command already supplies the correct operator-review posture, evidence verification foundations, audit surfaces, local APIs, and run lifecycles. It does not currently expose a verified Tarcie intake contract.

## 3. Goals

### Must have

- Sub-five-second note and screenshot initiation.
- Explicit beta-session identity and build/environment binding.
- Offline-first local capture with visible delivery state.
- Separate binary artifacts with SHA-256 hashes; no screenshot base64 in JSONL.
- Idempotent event and artifact submission.
- Per-item acceptance or rejection with reason codes.
- Typed, minimized, immutable evidence receipts.
- Localhost-only default transport.
- Forge_Command read-only Beta Evidence Inbox.
- DataForge Local as durable accepted-evidence storage without semantic authority.
- Fail-closed handling of unknown schemas, unknown session identity, hash mismatch, conflict, missing artifact, and unverifiable acknowledgment.
- Explicit privacy, redaction, retention, and deletion rules.
- Accessibility and keyboard-first capture.

### Should have

- Steps to reproduce and expected/observed fields.
- Screenshot annotation and deliberate redaction before submission.
- Session timeline and evidence grouping in Forge_Command.
- Exportable evidence bundle and replay-safe receipt chain.
- Application/window context captured only through explicit permission and allowlisting.

### Will not have in v0.1

- Autonomous testing or navigation.
- AI/LLM processing inside Tarcie.
- Automatic severity or defect verdicts.
- Automatic GitHub issue creation.
- Repair, code mutation, deployment, promotion, or rollback authority.
- Background screen recording, continuous surveillance, keystroke capture, or unrestricted filesystem access.
- Remote/cloud sink enabled by default.
- DataForge interpreting evidence or deciding status.
- Silent fallback from live evidence to demo/mock state.

## 4. Authority model

| Component | Authorized responsibility | Prohibited responsibility |
| --- | --- | --- |
| Tarcie | Capture operator-authored observations, explicit screenshots, markers, session metadata, local spool state | Diagnosis, repair, approval, promotion, autonomous operation |
| Forge_Command | Validate intake, issue receipts, present review state, record operator decisions, coordinate governed handoffs | Governance doctrine, unapproved side effects, inventing evidence |
| DataForge Local | Durable versioned storage for accepted artifacts, observations, and receipts | Categorization, verdicts, workflow authority |
| Forge:SMITH | Policy and governance evaluation for later handoffs | Rewriting source evidence |
| FA Local / approved executor | Execute only separately authorized bounded side effects | Expanding scope or self-authorizing |
| Human operator | Start/end session, capture, redact, submit, review, decide | None within owned decision scope |

## 5. Information classes

The UI and contracts must never collapse these classes:

1. **Operator observation:** what the operator entered or deliberately captured.
2. **Artifact fact:** immutable file identity, hash, dimensions, media type, and capture timestamp.
3. **Transport state:** local-only, queued, submitting, accepted, partially accepted, rejected, or deferred.
4. **Review annotation:** operator or authorized reviewer interpretation attached after intake.
5. **Derived finding candidate:** non-authoritative downstream proposal.
6. **Governed decision:** explicitly authorized lifecycle action with its own receipt.

## 6. Proposed contracts

### BetaSession.v1

Required fields: `session_id`, `product_id`, `application_id`, `build_id`, `environment`, `operator_id`, `device_id`, `started_at`, `source_versions`, `privacy_profile`, and `schema_version`.

Repository and commit SHA are optional only when the tested build has no repository binding. Their absence must be visible.

### BetaObservation.v1

Required fields: `observation_id`, `session_id`, `sequence`, `captured_at_utc`, `captured_at_mono_ms`, `observation_type`, `content`, `operator_asserted`, `artifact_refs`, `source_version`, and `schema_version`.

Allowed initial types: `note`, `marker`, `screenshot_observation`, `expected_observed`, and `reproduction_step`.

### BetaArtifact.v1

Required fields: `artifact_id`, `session_id`, `media_type`, `byte_length`, `sha256`, `captured_at`, `local_spool_ref`, `redaction_state`, `capture_scope`, and `schema_version`.

The transport contract carries artifact bytes separately. `local_spool_ref` is never trusted by Forge_Command as a remote path.

### BetaEvidenceSubmission.v1

Required fields: `submission_id`, `session_id`, ordered observation IDs, artifact manifest, schema versions, client version, idempotency key, submitted_at, and canonical payload hash.

### BetaEvidenceAcceptanceReceipt.v1

Required fields: `receipt_id`, `submission_id`, `session_id`, receiver identity/version, accepted observation IDs, accepted artifact IDs and hashes, rejected items with reason codes, durable-store references, received_at, canonical payload hash, receipt hash, and receipt schema version.

HTTP success without this receipt is not acceptance.

## 7. Fail-closed controls

### Zero-tolerance controls

- Artifact hash mismatch.
- Unknown or conflicting session identity.
- Schema version not admitted by the receiver.
- Duplicate ID with different content.
- Receipt that does not bind the submitted canonical payload hash.
- Capture outside the explicit user-selected scope.
- Secret/token leakage detected by deterministic preflight rules.
- Missing redaction decision when the privacy profile requires review.
- Claiming `accepted` without a verified acceptance receipt.

### Delta-gated controls

- Capture latency compared with the current Tarcie baseline.
- Queue/spool disk usage and retention behavior.
- Forge_Command intake and inbox performance.
- Screenshot file size and compression tradeoffs.
- Accessibility and keyboard-completion time.
- Retry rate, duplicate rate, and deferred-submission recovery.

Baselines may be recorded only through an explicit approved baseline operation. Candidate runs cannot silently rewrite them.

## 8. Privacy and security

- Screenshot capture is explicit per action, never continuous.
- Default capture scope is the active window chosen by the operator; whole-screen capture requires a separate action.
- Password managers, credential prompts, terminals, private messages, manuscript content, and other denylisted surfaces require block or explicit redaction policy.
- OCR, AI summarization, and automated content analysis are out of the first slice.
- Local spool content receives a defined retention period, storage cap, cleanup receipt, and encryption-at-rest decision before beta qualification.
- Remote transport remains disabled unless a later plan revision separately authorizes it.
- Logs contain identifiers, hashes, sizes, states, and reason codes—not screenshot pixels or unrestricted note content.

## 9. First proving slice

One Linux desktop, one deliberately selected test application, one manual beta session, and one localhost Forge_Command instance.

The slice includes:

- Start/end session.
- Text note and timestamp marker.
- Explicit active-window screenshot.
- Separate artifact file and SHA-256 manifest.
- Local spool with crash/restart recovery.
- Fixture-backed Forge_Command intake validator.
- Receipt generation using a non-production DataForge Local fixture or isolated test store.
- Read-only Forge_Command Beta Evidence Inbox showing observation, screenshot, delivery state, hashes, and rejection reasons.

The slice excludes cloud routing, background capture, AI, GitHub issue creation, agent execution, repair, and promotion.

## 10. Acceptance gates

### GATE-00 — Source lock and contract review

- Pin Tarcie, Forge_Command, DataForge Local, and applicable schema/protocol revisions.
- Reconcile the actual receiver surface and durable-store API.
- Validate schemas and negative fixtures.
- Board Review 1 authorizes only the proving slice.

### GATE-01 — Local capture proof

- Crash-safe note and screenshot spool.
- Hash verification and no screenshot-in-JSONL rule.
- Permission-denial, disk-full, malformed-record, and restart tests.
- Capture p95 remains within the approved latency threshold.

### GATE-02 — Intake and receipt proof

- Idempotent repeated submission.
- Partial-acceptance behavior is deterministic.
- No accepted state without hash-bound receipt.
- Unknown identity/schema/hash conflict fails closed.

### GATE-03 — Inbox truthfulness proof

- Live, fixture, replay, rejected, deferred, and unavailable states are visually distinct.
- Missing evidence never appears accepted.
- The inbox is projection-only and cannot mutate canonical evidence.

### GATE-04 — Security and privacy qualification

- Explicit capture scope, redaction, denylist, retention, cleanup, and logging tests pass.
- Threat model and abuse cases receive RED Board review.

### GATE-05 — Board Review 2 shadow qualification

- Representative beta sessions run in shadow/limited beta.
- Receipts, failure recovery, privacy, performance, and operator comprehension are independently reviewed.
- Any broader beta release requires an explicit operator authorization.

## 11. Stop conditions

Stop the work package immediately if:

- Screenshot capture requires unrestricted or persistent surveillance permissions.
- The receiver cannot return per-item hash-bound acceptance.
- Partial failure can lose or duplicate evidence without detection.
- A note or screenshot can be marked accepted without durable-store proof.
- The implementation moves workflow or semantic authority into DataForge Local.
- Tarcie grows a repair, approval, or autonomous execution path.
- Sensitive material enters logs, fixtures, or test artifacts.
- Live failure silently substitutes demo or cached evidence.
- Required source identity cannot be pinned.

## 12. Implementation order

1. Source lock and protocol reconciliation.
2. Versioned schemas and negative fixtures.
3. Acceptance receipt before broad feature work.
4. Local screenshot spool and hash manifest.
5. Forge_Command fixture intake and receipt verification.
6. Read-only evidence inbox.
7. Privacy, accessibility, retention, and fault qualification.
8. Board Review 2 limited beta decision.

## 13. Explicit non-authorization

This plan does not authorize implementation, repository mutation, GitHub changes, live DataForge writes, provider access, cloud deployment, beta-user enrollment, credential creation, background capture, agent execution, repair, or promotion.

