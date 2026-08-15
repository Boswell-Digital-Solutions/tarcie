# Editable PDF Session Report Specification

## Purpose

When the tester chooses End & Review, Tarcie displays a writable PDF report that
is already populated from the session. The report reduces tester workload and
produces a familiar handoff document without sacrificing structured evidence.

## Trust model

- Tarcie generates the PDF locally from a trusted, versioned template.
- The first slice does not load arbitrary user- or package-supplied PDF
  templates.
- PDF JavaScript, launch actions, embedded files, external auto-open links, and
  dynamic network content are prohibited.
- The PDF is human-facing. `BetaSessionReport.v1` plus the finalization manifest
  remain the canonical structured record.
- A PDF edit is canonical only after its admitted fields are extracted,
  validated, and rebound to the structured report.

## Report sections

1. Session identity: product, application, build, session, assignment, tester,
   Tarcie version, platform, start/end, and elapsed active time.
2. Coverage summary: assigned, reviewed, partial, blocked, skipped, not
   applicable, and remaining sections.
3. Section review: section state, what was tested, observations, blockers, and
   missing evidence explanation.
4. Findings: intended action, expected result, observed result,
   reproducibility, blocking status, evidence references, and tester priority.
5. Unresolved and free exploration notes.
6. Privacy review: included/excluded artifacts and redaction attestations.
7. Overall experience and tester sign-off.

## Field classes

### Read-only computed fields

Product/build/session identity, hashes, timestamps, durations, original evidence
IDs, artifact hashes, and package versions.

### Writable controlled fields

Coverage state from an admitted enumeration, section comments, expected and
observed narratives, reproducibility, blocking assessment, unresolved work,
overall experience, privacy acknowledgement, and tester attestation.

### Prohibited edits

Changing source evidence text, artifact bytes/hashes, build identity, original
capture time, or receiver status from the PDF.

## Field-map contract

Every field has a stable name, type, required flag, maximum length, source
classification, structured JSON pointer, and PDF widget location. The field map
version is stored in both the PDF metadata and `BetaSessionReport.v1`.

Dynamic product sections generate repeated field groups with names derived from
validated section IDs, never unchecked display text. Page growth is
deterministic and bounded.

## Save and round-trip

1. Tarcie populates fields and creates appearance streams.
2. The tester edits admitted fields.
3. Save Draft writes atomically inside encrypted app-owned storage.
4. Tarcie reopens the file and verifies the AcroForm field tree, page widgets,
   values, and non-empty appearances.
5. Values are validated and written to `BetaSessionReport.v1`.
6. The PDF is regenerated or normalized from the canonical field set.
7. Finalization hashes the final PDF and structured report together.

Missing fields, duplicate names, orphaned widgets, stale values, invalid
enumerations, empty required values, missing appearances, or failed reopen stop
closed.

## Viewer behavior

The implementation must qualify one controlled in-app PDF component on
Windows, macOS, and Linux. Opening an external system viewer may be offered as
a copy workflow but cannot be the only canonical edit path unless round-trip
behavior is independently qualified.

## Accessibility

- Tagged document structure or an equivalent accessible structured report
  navigator is required.
- Fields have programmatic labels, descriptions, required state, and logical
  tab order.
- Multiline fields remain legible and scrollable.
- All report content is available outside the visual page canvas.
- Final output is visually inspected for clipping, overlap, unreadable text,
  broken glyphs, and hidden field values.

## Documentation prototype

`templates/Beta_Session_Report_v1_FILLABLE_PROTOTYPE.pdf` is a non-operative
AcroForm prototype containing synthetic fields. It demonstrates the proposed
interaction and is not an implementation, production template, or evidence
record.

