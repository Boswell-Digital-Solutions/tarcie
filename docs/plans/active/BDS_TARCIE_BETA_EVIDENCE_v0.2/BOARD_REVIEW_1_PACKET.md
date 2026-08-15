# Board Review 1 Packet - BDS-TARCIE-BETA-EVIDENCE-v0.2

## Decision requested

Authorize, rework, or reject only GATE-00 current-source lock and
documentation-only contract/fixture/threat-model/PDF-prototype work for the
superseding v0.2 candidate.

## Product proposal

Tarcie becomes a cross-platform guided field companion for human beta testers.
Forge_Command prepares one bounded `.tarcie-session` assignment. Start Session
shows the exact product/build and assigned sections. Tarcie captures reviewed
observations while tracking coverage. End & Review opens a writable PDF report
already populated from the session. Finalize binds the PDF, structured report,
observations, artifacts, and hashes into a local candidate evidence package.

Tarcie remains non-autonomous. It does not navigate Author_Forge, diagnose,
assign authoritative severity, repair, deploy, promote, or automatically create
GitHub issues.

## Required Board determinations

### Architecture

- Is the separate Session Hub compatible with the fast overlay boundary?
- Is the `.tarcie-session` package sufficiently non-executable and bounded?
- Is Forge_Command the correct package/lifecycle/receiver owner?
- Is the PDF correctly treated as a structured projection rather than sole
  evidence authority?

### RED Board

- Can packages escape paths, carry active content, leak secrets, or confuse
  product/build identity?
- Can screenshots capture too much or preserve raw pixels?
- Can PDF active features, parser faults, duplicate fields, or stale values
  falsify the report?
- Can retries or partial failure lose, duplicate, or falsely accept evidence?

### Evidence and Receipt Board

- Does finalization bind the exact PDF, structured report, coverage, evidence,
  and artifact hashes?
- Are original observations immutable and corrections explicit?
- Is every accepted item tied to a hash-bound receipt and durable-store proof?

### Privacy and Accessibility

- Can testers understand and control capture, exclusion, redaction, local state,
  delivery state, and finalization?
- Is the workflow usable without color, pointer-only canvas interaction, or
  inaccessible PDF fields?

### Operations and Platform

- Are Windows, macOS, and Linux treated as installed qualification targets for
  both Tarcie and Author_Forge?
- Are platform-specific integration limits kept separate from application
  support claims?
- Can Pause/Resume, restart, PDF save/reopen, update, and uninstall recover
  without evidence loss?

## Recommended disposition

**AUTHORIZE FOR GATE-00 AND CONTRACT/FIXTURE WORK ONLY, WITH FAIL-CLOSED
CONDITIONS.**

## Exact authorization wording

> AUTHORIZE `BDS-TARCIE-BETA-EVIDENCE-v0.2` for GATE-00 current-source lock and
> documentation-only Phase 1 contract, schema, canonicalization, fixture,
> threat-model, wireframe, PDF-prototype, and cross-platform qualification-plan
> work. This authorization permits no runtime implementation, repository
> mutation, screenshot permission, personal or production content ingestion,
> live beta enrollment, production DataForge write, Forge_Command delivery,
> network or cloud transport, credentials, deployment, automatic issue
> creation, autonomous testing, diagnosis, repair, promotion, or release claim.
> Any code work requires a separate operator-authorized work package bound to
> the completed GATE-00 ledger and approved schemas.

## Rejection triggers

Reject or return for rework if the package can execute content; receiver/storage
ownership remains ambiguous; writable PDF values cannot round-trip without
loss; PDF active content is required; screenshot scope cannot fail closed;
original evidence can be silently changed; platform support is inferred from
build artifacts; or Tarcie gains autonomous, issue, repair, or promotion
authority.

