# Schema and Contract Specification

## Identity rules

- Every object carries `schema_version` and a globally unique identifier.
- IDs are immutable and never reused across products, builds, devices, or
  revisions.
- Dates use UTC RFC 3339; elapsed duration uses a monotonic counter.
- Repository commit is required when a tested build has a repository binding.
- Duplicate ID plus different canonical hash is a hard conflict.
- Unknown schema versions fail closed.

## ProductBetaProfile.v1

Defines stable product structure, not one tester's work:

- `profile_id`, `product_id`, `application_id`, `display_name`
- `profile_version`, `supported_platforms`, `sections`
- per-section `section_id`, name, purpose, guardrails, and evidence prompts
- admitted guidance attachments and privacy categories
- profile canonical hash

The profile cannot start a session and contains no tester identity, secret, or
credential.

## BetaSessionAssignment.v1

Defines one bounded assignment:

- `assignment_id`, `profile_id`, `session_id`, `tester_id`
- `product_id`, `application_id`, exact `build_id`
- optional repository and required commit when applicable
- `platform_policy`, assigned section IDs, required/optional flags
- free-exploration allowance, time window, privacy profile
- issue/report prompts, support contact reference, expiry

## TarcieSessionBundleManifest.v1

Defines a `.tarcie-session` archive:

- bundle ID/version, assignment and profile filenames
- ordered file manifest with path, media type, byte length, and SHA-256
- canonical payload hash, created time, expiry, producer identity
- optional signature block reserved for an approved key lifecycle

The first slice admits JSON, UTF-8 plain text/Markdown, PNG/JPEG reference
images, and a trusted template identifier. HTML, JavaScript, executables,
symlinks, absolute paths, traversal segments, embedded credentials, and external
URLs that auto-open are prohibited.

## BetaSession.v1

- `session_id`, `assignment_id`, `product_id`, `build_id`
- tester/device identities, platform fingerprint, Tarcie version
- state, started/paused/resumed/reviewing/finalized timestamps
- active and elapsed monotonic durations
- loaded bundle and profile hashes
- ordered section states and sequence ceiling

## BetaObservation.v1

- observation ID, session ID, section ID or `free_exploration`
- sequence, UTC and monotonic capture times
- type: note, marker, expected_observed, reproduction_step,
  screenshot_observation, or section_comment
- original operator-authored content, artifact references
- operator-asserted flag and source version

Corrections use `BetaObservationCorrection.v1` and never overwrite the original.

## BetaArtifact.v1

- artifact ID, session ID, observation ID
- media type, byte length, SHA-256, dimensions where applicable
- capture time/scope, redaction state, final-derivative attestation
- app-owned storage reference that is never trusted as a receiver path
- cleanup and retention state

## BetaSectionCoverage.v1

- session and section IDs
- state: not_started, in_progress, reviewed, partial, blocked, skipped,
  not_applicable
- started/completed times, observation IDs, tester comment
- required missing-evidence reasons and blocker reference

## BetaSessionReport.v1

- report ID/revision, session/assignment/product/build identity
- tester and platform identity
- immutable start/end/duration facts
- ordered section coverage
- finding summaries with original evidence references
- unresolved work, excluded evidence, privacy review, overall experience
- tester attestation, report state, PDF field-map version
- structured report canonical hash and generated PDF SHA-256

Only narrative/report fields are editable. Session identity, evidence hashes,
times, and source IDs are computed and displayed read-only.

## BetaEvidenceSubmission.v1

Contains ordered report, observation, artifact, and manifest identities plus an
idempotency key and canonical payload hash. It contains no trusted remote paths.

## BetaEvidenceAcceptanceReceipt.v1

Contains receiver identity/version, submission and session IDs, accepted and
rejected item IDs with reason codes, durable-store references, canonical payload
hash, receipt hash, and time. HTTP 2xx without this receipt is not acceptance.

## Canonicalization

Canonical JSON uses UTF-8, object keys sorted lexicographically, no insignificant
whitespace, arrays in defined order, normalized RFC 3339 UTC timestamps, and no
NaN/Infinity. Hashes use lowercase SHA-256 hex. For `BetaSessionReport.v1`, the
structured-report hash input omits `structured_report_sha256` and
`generated_pdf_sha256`; the completed report stores those resulting bindings.
Exact test vectors must be approved before implementation.
