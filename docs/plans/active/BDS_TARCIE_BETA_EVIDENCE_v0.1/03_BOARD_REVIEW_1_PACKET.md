# Board Review 1 Packet — BDS-TARCIE-BETA-EVIDENCE-v0.1

## Decision requested

Review the architecture and authorize, rework, or reject only Phase 0 source lock and Phase 1 fixture/contract work. Do not authorize live screenshot capture, production persistence, beta enrollment, cloud transport, agents, repair, or promotion.

## Proposed disposition

**REWORK UNTIL GATE-00 EVIDENCE IS COMPLETE.** The concept is sound and aligned with existing product boundaries, but the current Forge_Command Tarcie receiver and DataForge Local persistence contract are unverified.

## Board questions

### Architecture Board

- Does the plan extend Tarcie and Forge_Command without creating a competing evidence service?
- Is the intake owner Forge_Command, or should a pre-existing local evidence API own the receiver?
- Are session, observation, artifact, submission, and receipt identities sufficient for deterministic replay?

### RED Board

- Can screenshot permissions become surveillance or capture the wrong window?
- Can secrets, private messages, terminals, manuscripts, or credentials enter artifacts or logs?
- Can retry/partial failure create undetected duplicates or evidence loss?
- Can a forged `2xx`, receipt, local path, device ID, or session ID create false acceptance?
- Can disk exhaustion or malformed JSONL destroy later evidence?

### Evidence and Receipt Board

- Does every accepted item bind to the exact submitted hash?
- Is partial acceptance deterministic and replay-safe?
- Can storage proof be separated from semantic review?
- Are canonicalization, schema versioning, and supersession rules explicit?

### Privacy and Accessibility Board

- Is capture explicit, obvious, interruptible, and bounded to an operator-selected scope?
- Are redaction, retention, cleanup, and encryption decisions adequate?
- Can every state be understood without color and operated by keyboard?

### Operations Board

- Can the system recover after Tarcie, Forge_Command, or DataForge Local stops mid-submission?
- Are spool caps and cleanup behavior deterministic?
- Are unavailable and deferred states visible without silent fallback?

## Evidence required to close Board Review 1

- Current commit-pinned source ledger.
- Actual Tarcie test/build result and queue failure analysis.
- Verified Forge_Command receiver ownership decision.
- Verified DataForge Local persistence boundary.
- Draft schemas and canonical hash vector.
- Positive and negative fixtures.
- Threat model for screenshot capture and artifact transport.
- Acceptance receipt contract with idempotency and partial-failure semantics.

## Exact bounded authorization wording

> AUTHORIZE `BDS-TARCIE-BETA-EVIDENCE-v0.1` Phase 0 source lock and Phase 1 documentation, schema, canonicalization, fixture, threat-model, and receipt-contract work only. This authorization permits no live screenshot capture, production or personal-content ingestion, DataForge production writes, cloud or remote transport, credential creation, repository merge, agent execution, automatic issue creation, repair, promotion, deployment, or beta-user enrollment. Any implementation proving slice requires a separate post-GATE-00 authorization bound to pinned repository commits and approved schemas.

## Rejection triggers

- Receiver or persistence ownership remains ambiguous.
- Screenshot capture cannot be permission-bounded.
- Acceptance cannot be proven with a hash-bound per-item receipt.
- Sensitive-content minimization cannot be tested deterministically.
- The design grants Tarcie semantic, repair, or promotion authority.

