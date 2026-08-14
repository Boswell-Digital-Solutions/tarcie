# tarcie Extended Roadmap

**Document version:** 1.1 (2026-08-14) — verification hardening delivered

## Current Status

| Phase | Status | Outcome |
| --- | --- | --- |
| Documentation normalization | Complete | Protocol-required baseline surfaces are present |
| Verification hardening | Complete | 52 unit tests and a CI workflow that runs them |
| Module catalog expansion | Pending | Expand exact routes, tables, and runtime contracts from code |

## Phase 0 — Documentation Normalization

**Goal:** establish the required documentation stack and build surfaces.

**Delivered:**

- root `CLAUDE.md`
- modular `doc/system/`
- `doc/system/BUILD.sh`
- `doc/TARSYSTEM.md`
- `scripts/context-bundle.sh`

## Phase 2 — Verification Hardening

**Goal:** align repo-specific testing, QA, and handover documentation with
current implementation reality.

**Delivered:**

- 52 unit tests across `queue/jsonl.rs`, `ipc/commands.rs`, `sink/config.rs`,
  and `flusher.rs`
- `.github/workflows/ci.yml`, which runs the tests and the document build on
  every pull request and fails on a stale `doc/TARSYSTEM.md`
- corrections for six defects the suite exposed, including the two that broke
  the durability contract

Section 10 of the system document records what the tests cover and what they
do not. The uncovered set is the honest starting point for more work here.

## Phase 1 — Exact Surface Expansion

**Goal:** replace baseline placeholders with exact module, API, schema, and
environment documentation.

This is the remaining phase. The runtime chapters (sections 4 to 6) are now
exact, because the durability work rewrote them against the code. The
configuration and command chapters have not had the same treatment.
