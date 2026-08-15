# BDS-TARCIE-BETA-EVIDENCE-v0.2 - Plan Set Index

**Title:** Tarcie Guided Beta Sessions, Preloaded Assignments, and Writable Closeout Reports  
**Plan ID:** `BDS-TARCIE-BETA-EVIDENCE-v0.2`  
**Prepared:** 2026-08-15  
**Decision owner:** Charlie Boswell / Boswell Digital Solutions  
**Lifecycle:** `proposed_for_board_review_1`  
**Authority:** documentation-only candidate; implementation not authorized  
**Lineage:** proposed successor to `BDS-TARCIE-BETA-EVIDENCE-v0.1`; v0.1 remains controlling until this revision is authorized  
**Primary repository:** `Boswell-Digital-Solutions/tarcie`  
**Primary product under test:** `Boswell-Digital-Solutions/Author-Forge`  
**Control plane:** `Boswell-Digital-Solutions/Forge_Command`  
**Candidate durable store:** `Boswell-Digital-Solutions/dataforge-Local`  
**Governance authority:** Forge:SMITH plus applicable BDS protocols

## Executive decision requested

Admit a cross-platform Tarcie session workflow for human beta testers:

1. Forge_Command prepares a bounded `.tarcie-session` package for one product,
   exact build, tester assignment, and set of product sections.
2. The tester loads the package into Tarcie on Windows, macOS, or Linux.
3. **Start Session** presents the assignments, sections, guidance, remaining
   work, privacy boundaries, and optional free-exploration allowance.
4. Tarcie keeps its fast capture overlay while a separate Session Hub tracks
   the active section and coverage.
5. **End & Review** opens a locally generated writable PDF report whose fixed
   facts are pre-populated and whose narrative fields remain editable.
6. **Finalize Session** creates a human-readable PDF, structured
   `BetaSessionReport.v1`, evidence manifest, and candidate submission package.
7. Nothing is accepted, turned into a GitHub issue, diagnosed, repaired, or
   promoted without a separate governed receipt and human decision.

## Binding requirements

- Tarcie is an easy, guided companion for human beta testers. It is not the
  autonomous Beta by Forge campaign system.
- Tarcie and Author_Forge must both be installable and operational on Windows,
  macOS, and Linux. A build artifact alone is not a support claim.
- A tester receives one ordinary application installer per product plus a
  separate session package. Tarcie is never rebuilt per tester or assignment.
- The existing 480 x 140 fast-capture overlay remains a distinct surface; the
  Session Hub and writable closeout report do not consume it.
- Buttons, admitted `#action` tags, and focused shortcuts resolve through one
  governed action engine. Context tags never execute.
- Original tester observations are immutable after capture. Corrections,
  redactions, review annotations, and conclusions are attached without silently
  rewriting source evidence.
- The PDF is a human-facing writable report bound to structured fields. It is
  not the sole canonical evidence store.
- Offline/local operation is the first trust boundary. Network delivery and
  production persistence remain separately gated.

## Current source observations

- Tarcie `main` was connector-observed at
  `7ad58c601312baaf9880bc3120a47e1b77bb34ed` on 2026-08-15. Its current UI is
  write-only, with notes, markers, a JSONL queue, and three IPC commands. PR #20
  reports 106 Rust tests and 29 frontend tests; GATE-00 must reproduce claims.
- Tarcie's current bundle targets are Linux `.deb` and AppImage. Windows and
  macOS packaging and installed qualification are open gaps.
- Author_Forge `main` was connector-observed at
  `5b7765113ead1465f4d8a3802abfc7e34e8a9b07`. Its release workflow contains
  Linux x86_64, macOS Apple Silicon, and Windows x86_64 jobs, but installed
  feature qualification remains evidence-gated.
- Forge_Command `main` was connector-observed at
  `87fd7f11ad088079949f15a17752c93c29782fdb`.
- dataforge-Local `master` was connector-observed at
  `6c07d13d31a6f6c07789cc23429d1b474ec0816a`.

These are source observations, not independent clean-build receipts.

## File index

1. `00_README_AND_INDEX.md` - controlling index and authority statement.
2. `01_BOARD_FINDINGS_AND_DECISIONS.md` - Board findings, conflicts, and exact
   disposition requested.
3. `02_MASTER_IMPLEMENTATION_PLAN.md` - gated work packages and stop conditions.
4. `03_SCHEMA_CONTRACT.md` - versioned identities, packages, reports, and
   acceptance receipts.
5. `04_SESSION_PACK_AND_UX_SPEC.md` - prepared assignment, Start Session,
   active capture, End & Review, and Finalize flows.
6. `05_CROSS_PLATFORM_QUALIFICATION.md` - Windows, macOS, and Linux release
   contract for both Tarcie and Author_Forge.
7. `06_EDITABLE_PDF_REPORT_SPEC.md` - writable PDF behavior, field binding, and
   cross-platform validation.
8. `07_FORGE_COMMAND_DATAFORGE_INTEGRATION.md` - authority, delivery, receipt,
   and storage separation.
9. `08_SECURITY_PRIVACY_THREAT_MODEL.md` - trust boundaries, threats, and
   fail-closed controls.
10. `09_CI_EVIDENCE_AND_ACCEPTANCE.md` - CI matrix, fixtures, qualification
    evidence, and release gates.
11. `10_AGENT_PROMPTS.md` - bounded implementation and review prompts.
12. `11_SOURCE_AND_LINEAGE_BASELINE.md` - pins, v0.1 lineage, and claim limits.
13. `BOARD_REVIEW_1_PACKET.md` and `review_packet.pdf` - exact BR1 packet.
14. `plan_set.yaml`, `MANIFEST.yaml`, `REGISTRY_ENTRY.json`,
    `FOLDER_METADATA.json`, `evidence_index.yaml`, `PDF_VALIDATION_REPORT.json`,
    `PACKAGE_VALIDATION_REPORT.json`, and `CHECKSUMS.sha256` - machine-readable
    controls and validation results.
15. `schemas/` - strict candidate JSON Schemas.
16. `fixtures/` - non-operative Author_Forge session-package example.
17. `templates/Beta_Session_Report_v1_FILLABLE_PROTOTYPE.pdf` - interactive
    documentation prototype, not runtime implementation.
18. `templates/Beta_Session_Report_v1_FIELD_MAP.json` - binding between PDF
    field names and candidate structured-report properties.

## Authority and next gate

This packet requests Board Review 1 of the revised architecture and authority
boundaries. It does not inherit v0.1 implementation authority because v0.1
granted none. If BR1 authorizes bounded planning work, GATE-00 must create a
clean, immutable, current source ledger and reconcile the exact receiver,
storage, packaging, PDF, screenshot, and platform surfaces before any code work
package may be proposed.
