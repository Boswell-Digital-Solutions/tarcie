# Forge_Command and DataForge Local Integration

## Authority separation

| Component | Owns | Must not own |
| --- | --- | --- |
| Forge_Command | Product profiles, assignment preparation, lifecycle authorization, intake validation, receipts, operator review, governed handoffs | Source-evidence rewriting, unapproved repository mutation |
| Tarcie | Package validation, human guidance, local session state, reviewed observations, explicit artifacts, writable closeout | Autonomous navigation, diagnosis, severity authority, repair, promotion |
| DataForge Local | Durable versioned storage for accepted objects and receipt references | Classification, defect verdict, workflow authority |
| Forge:SMITH | Policy evaluation, Board Review presentation, promotion gates | Altering source observations |
| Human tester | Start/Pause/End, capture, correction, redaction, exclusion, report completion, final approval | Receiver acceptance or repository authority |
| Charlie Boswell | Work-package, promotion, release, and policy decisions | None within owned authority |

## Package preparation

Forge_Command may emit a `.tarcie-session` package only after validating the
product profile, build identity, assignment, privacy profile, section set,
attachments, limits, and expiry. Package creation produces its own receipt and
does not enroll a tester or start a session.

## Delivery separation

Capture, finalization, submission, acceptance, and issue proposal are distinct
transactions:

1. Capture produces local candidate evidence.
2. Finalize binds report and evidence into a local immutable package.
3. Submit attempts delivery with an idempotency key.
4. Accept validates per item and returns a hash-bound receipt.
5. Review may create a candidate finding.
6. A separately governed action may propose or create a GitHub issue.

No earlier state implies a later one.

## Receiver rules

- Localhost-only default.
- Exact admitted schema versions.
- Package/session/build identity match.
- Canonical payload and artifact hash verification.
- Replay-safe idempotency.
- Deterministic partial acceptance.
- Explicit reason codes.
- No accepted state before durable-store proof.
- No trusted client path or client-provided acceptance state.

## DataForge Local boundary

Accepted structured objects, immutable artifacts, and receipt references may be
stored through a bounded interface after that interface is identified and
authorized. DataForge Local records facts and versions; it does not decide
severity, validity, priority, issue state, or promotion.

## Forge_Command Inbox

The read-only Beta Evidence Inbox distinguishes:

- live, fixture, and replay data;
- local-only, queued, submitting, partial, accepted, rejected, deferred, and
  unavailable delivery states;
- original observation, correction, reviewer annotation, candidate finding,
  and governed decision;
- platform and integration qualification claims;
- PDF work copy, finalized PDF, and structured report.

No missing evidence appears accepted. No cached or demo state silently replaces
live failure.

