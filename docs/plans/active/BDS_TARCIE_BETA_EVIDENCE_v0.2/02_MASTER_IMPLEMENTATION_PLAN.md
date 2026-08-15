# Master Implementation Plan

## Governing posture

Progress is gate-based, not date-based. Work may stop between any two gates
without leaving a partially authorized product claim. At most one exact
implementation work package is authorized at a time.

## Phase 0 - GATE-00 current-source lock

**Goal:** replace connector observations with independently reproducible source
and build evidence.

- Pin Tarcie, Author_Forge, Forge_Command, dataforge-Local, contract libraries,
  Tauri, PDF component, and packaging tool revisions.
- Reproduce Tarcie and Author_Forge clean builds in bounded environments.
- Record existing tests, missing tests, capabilities, IPC surfaces, CSP,
  filesystem boundaries, queue behavior, and packaging outputs.
- Resolve receiver and durable-store ownership in writing.
- Record current installer behavior on Windows, macOS, and Linux without making
  support claims from unqualified artifacts.
- Reconcile v0.1, A1-R2, the 2026-08-14 BR1 decision, and this candidate.

**Exit:** signed GATE-00 source ledger and a separately authorized first work
package. No source ledger may be silently refreshed by a candidate build.

## Phase 1 - Contract and fixture spine

**Goal:** prove the workflow without real capture or a live tester.

- Finalize strict schemas for product profiles, assignments, package manifests,
  sessions, sections, observations, artifacts, reports, submissions, and
  receipts.
- Define canonical JSON serialization and SHA-256 vectors.
- Build positive, negative, conflict, replay, corruption, oversize, traversal,
  and unsupported-platform fixtures.
- Define the trusted writable-PDF template and field map.
- Prove PDF form-field extraction, save/reopen, and structured round-trip using
  synthetic values only.
- Produce accessible wireframes and state-transition truth tables.

**Exit:** schema proof, fixture report, threat model, field-map proof, and BR1
closeout. Still no runtime implementation authority.

## Phase 2 - Session package loader

**Goal:** load and display one non-executable synthetic session package.

- Add a separate Session Hub.
- Validate extension, archive shape, normalized paths, schema versions, hashes,
  size limits, attachment types, product/build identity, and expiry.
- Display product, build, tester assignment, section cards, guardrails, and
  optional free exploration.
- Refuse unknown schemas, executable content, missing hashes, duplicate IDs,
  path escape, symlinks, credentials, or platform mismatch.
- Persist only an encrypted, app-owned synthetic session state.

**Exit:** loader and restart tests on all three platforms with no evidence
capture enabled.

## Phase 3 - Session lifecycle and section coverage

**Goal:** implement `READY -> ACTIVE -> PAUSED -> REVIEWING -> FINALIZED` with
crash-safe recovery.

- Start Session requires visible product/build/assignment confirmation.
- Exactly one session may be active.
- The Session Hub tracks current section, remaining sections, blockers, and
  free-exploration observations.
- Pause stops elapsed active time but does not finalize or discard drafts.
- End & Review stops ordinary capture only after durable closeout state exists.
- Reopen Review returns to the same report and evidence selection.
- Finalize is idempotent and irreversible except through an explicit superseding
  report revision.

**Exit:** state-machine, idempotency, crash, power-loss, and clock tests.

## Phase 4 - Guided observation and explicit artifacts

**Goal:** bind fast human capture to session and section identities.

- Preserve the current quick note/marker latency path.
- Route buttons, reserved action tags, and focused shortcuts through one typed
  backend action registry.
- Add explicit platform-qualified screenshots only after separate authorization.
- Require preview, annotation/redaction decision, and explicit Save.
- Preserve original observation text; corrections and review notes are separate.
- Keep artifact bytes separate from JSONL; bind them by SHA-256.

**Exit:** GATE-CAPTURE privacy, accessibility, durability, and performance proof
on Windows, macOS, and Linux.

## Phase 5 - Writable closeout report

**Goal:** display an editable, locally generated report when End & Review is
selected.

- Populate immutable facts from the session and evidence manifest.
- Permit edits only to admitted narrative, status, attestation, and exclusion
  fields.
- Keep one canonical field map shared by the report UI, PDF AcroForm, and
  `BetaSessionReport.v1`.
- Save, close, reopen, and continue without field loss.
- Generate the PDF from a trusted bundled template; refuse arbitrary templates.
- Bind final PDF bytes and structured report hash in the finalization manifest.

**Exit:** PDF logical and visual validation on qualified viewers for all three
platforms, plus round-trip and tamper tests.

## Phase 6 - Localhost receipt handoff

**Goal:** submit a finalized synthetic package to a fixture-backed
Forge_Command receiver.

- Use idempotency keys and ordered manifests.
- Verify every artifact and report hash before acceptance.
- Return per-item accepted/rejected states and hash-bound receipts.
- Persist through an isolated DataForge Local fixture only.
- Distinguish local, queued, submitting, partial, accepted, rejected, deferred,
  and unavailable states.

**Exit:** GATE-RECEIPT fault-injection report. No production writes.

## Phase 7 - Three-platform installed qualification

**Goal:** prove the complete bounded workflow in installed packages.

- Clean install, first launch, session load, Start, capture, Pause/Resume,
  End & Review, PDF edit/save/reopen, Finalize, restart recovery, update, and
  uninstall on Windows, macOS, and Linux.
- Run the same canonical synthetic session package and compare normalized
  outputs across platforms.
- Record platform differences without silently changing the contract.

**Exit:** one installed-qualification receipt per platform and a cross-platform
equivalence report.

## Phase 8 - Limited human beta and Board Review 2

**Goal:** shadow-qualify the real workflow against Author_Forge with synthetic
or disposable data.

- Human testers use actual installed surfaces.
- No real manuscripts in the first proving program.
- Measure completion, abandonment, capture latency, evidence loss, duplicate
  prevention, PDF comprehension, privacy incidents, and recovery.
- Produce a consensus report without automatically mutating repositories.

**Exit:** Board Review 2 recommendation of AUTHORIZE, REWORK, or REJECT.

## Global stop conditions

Stop immediately on evidence loss, duplicate acceptance, path escape, executable
session content, widened screenshot scope, surviving unredacted source pixels,
secret leakage, unbounded storage, unsupported schema acceptance, PDF field
loss, platform claim without installed proof, silent fallback, authoritative
DataForge interpretation, automatic GitHub mutation, or any self-expansion of
Tarcie into an autonomous campaign controller.

