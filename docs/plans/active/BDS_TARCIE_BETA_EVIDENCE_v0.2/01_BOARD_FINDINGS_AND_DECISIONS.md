# Board Findings and Decisions

## Proposed disposition

**AUTHORIZE FOR GATE-00 AND CONTRACT/FIXTURE WORK ONLY, WITH FAIL-CLOSED
CONDITIONS.**

The product concept is coherent and materially improves Tarcie's value to a
human beta tester. The revision is not implementation-ready because current
Tarcie is intentionally write-only, its distribution is Linux-only, the
writable-PDF runtime is absent, and receiver/storage ownership still requires
current source proof.

## Architecture Board findings

1. A separate Session Hub preserves Tarcie's sub-five-second capture overlay.
   Expanding the current 480 x 140 overlay into an assignment/report dashboard
   would violate its strongest existing constraint.
2. The session package must be data, not executable content. It may contain
   admitted JSON, plain text/Markdown guidance, and hashed static attachments;
   it may not contain JavaScript, HTML execution, binaries, credentials, or
   arbitrary filesystem paths.
3. Forge_Command owns session preparation, lifecycle authority, and receipt
   issuance. Tarcie owns guided interaction and local capture. DataForge Local
   may store accepted evidence but may not classify or decide it.
4. The writable PDF is a projection of `BetaSessionReport.v1`. Direct edits
   must round-trip through the same field map or remain visibly marked as an
   unimported human work copy.
5. Tarcie and Author_Forge are cross-platform products. Platform-specific
   integrations may have separate qualification claims, but an integration
   limitation must not be rewritten as an application-level platform limit.

## RED Board findings

- A session package can become a prompt-injection or arbitrary-file-delivery
  vehicle unless its schema, file types, paths, hashes, and size are validated.
- Product guidance may trick a tester into exposing secrets or private
  manuscripts. Packages require denylisted content categories and a visible
  privacy profile.
- Writable PDFs can carry JavaScript, actions, launch links, embedded files,
  or stale field appearances. Tarcie must generate reports from a trusted local
  template and refuse arbitrary PDF templates in the first slice.
- Cross-platform screenshot capture can widen scope, capture the wrong display,
  or preserve unredacted source pixels. Every platform requires explicit user
  action, preview, redaction decision, and verified cleanup.
- End Session can cause evidence loss if it stops capture before durable closeout
  state exists. The action therefore enters `REVIEWING`; only Finalize closes.
- A PDF must never be treated as proof that evidence was accepted. Acceptance
  requires a hash-bound receiver receipt.

## Evidence and Receipt Board findings

- Every session package binds product, application, exact build, assignment,
  platform policy, package hash, and schema versions.
- Coverage states are explicit: `not_started`, `in_progress`, `reviewed`,
  `partial`, `blocked`, `skipped`, and `not_applicable`.
- Original observations are immutable. A correction points to the original ID
  and records author, reason, time, and replacement narrative.
- Finalization binds the PDF bytes, structured report, observation IDs,
  artifact hashes, section coverage, and tester attestation in one manifest.
- Receiver acceptance remains per item. HTTP success without a valid receipt is
  delivery state only, never acceptance.

## Privacy and Accessibility Board findings

- All session tasks, report fields, controls, and state changes must be usable
  by keyboard and screen reader without requiring canvas manipulation.
- Status cannot be color-only. Each section displays text plus icon/state.
- The tester must be able to inspect, correct, redact, exclude, and approve
  every item before Finalize.
- The Session Hub must explain what is being captured, what stays local, what
  will be included, and what has not been delivered.
- PDF form fields require labels, logical tab order, multiline support, and
  visible values across qualified viewers on all three platforms.

## Operations Board findings

- One installer per supported platform plus one portable session package is the
  correct distribution model. Per-tester application builds are prohibited.
- Pause, restart, and receiver unavailability must preserve the session.
- No platform is supported until install, launch, session load, capture,
  closeout, PDF save/reopen, finalization, recovery, update, and uninstall are
  evidenced on that operating system.
- Build-matrix green is necessary but insufficient for a support claim.

## Unresolved decisions before implementation

1. Exact Forge_Command package-authoring and intake owner.
2. Exact DataForge Local accepted-evidence persistence contract.
3. Encryption-at-rest boundary for active sessions, drafts, screenshots, and
   report forms.
4. Trusted PDF rendering/editing component and its update policy.
5. Platform-specific screenshot target APIs and qualified scopes.
6. Release signing/notarization and update-channel operations for Tarcie.
7. Minimum supported OS versions and architectures.
8. Numeric limits for bundle size, attachment count, screenshot dimensions,
   PDF length, active-session duration, and spool totals.

## Exact bounded authorization requested

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

