# tarcie Architecture Spec

**Document version:** 1.1 (2026-08-14) — reconciled with the working tree

## 1. Purpose

This baseline architecture spec establishes a protocol-compliant design reference for tarcie.
It records only repository surfaces directly observable from the current working tree.

## 2. Current Implementation State

| Surface | Current truth |
| --- | --- |
| Canonical technical reference | `doc/system/` plus generated `doc/TARSYSTEM.md` |
| Repo-local instructions | `CLAUDE.md` |
| Current maturity | 93 Rust unit tests and 26 frontend unit tests; CI runs both suites, a TypeScript typecheck, and a stale-document check on every pull request |

## 3. Module Map

| Module | Surface | Current role |
| --- | --- | --- |
| Documentation stack | `doc/system/`, generated `doc/TARSYSTEM.md`, `scripts/context-bundle.sh` | Canonical repo context and build surfaces |
| Rust runtime | `src-tauri/src/` | IPC capture commands, JSONL queue, HTTP sink, background flusher, operational log |
| Overlay | `src/` | The capture window: DOM wiring, capture decisions, and the five-second revert |
| Planning and specs | `docs/` | This spec, the extended roadmap, and the governed plan set under `docs/plans/` |

The previous revision of this table listed `app/`, `service/`, `cortex_runtime/`,
`api/`, `crates/`, `governance/`, `DECISIONS/`, `prompts/`, `evals/`,
`analytics/`, and `registry/`. None of them exists in this repository. That row
was registry-generated boilerplate rather than observed repo truth.

## 4. Architectural Boundary

- this document is a baseline and must be expanded as concrete modules, routes, schemas, and integrations are cataloged
- when this spec and `doc/TARSYSTEM.md` diverge, `doc/TARSYSTEM.md` wins as the implemented reality reference

The previous revision named `SYSTEM.md` as that reference. There is no
`SYSTEM.md`: the root and `doc/` copies were retired, and `doc/TARSYSTEM.md` is
the assembled artifact.
