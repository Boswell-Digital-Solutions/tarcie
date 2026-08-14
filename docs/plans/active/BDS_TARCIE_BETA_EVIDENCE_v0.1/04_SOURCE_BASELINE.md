# Source Baseline — BDS-TARCIE-BETA-EVIDENCE-v0.1

## GitHub sources inspected

### Tarcie

- Repository: `Boswell-Digital-Solutions/tarcie`
- Default branch: `main`
- Inspected head: `309a231b0ce9ee7c1de88136d1d07356ffdfe93d`
- Head date: 2026-07-29
- Current implemented scope: Tauri/Rust/TypeScript note and marker capture; JSONL queue; generic HTTP sink; localhost-only default; three IPC commands.
- Known gaps: no screenshot capture, session identity, artifact contract, receiver acknowledgment receipt, readback, encryption-at-rest decision, or meaningful automated test suite.

### Forge_Command

- Repository: `Boswell-Digital-Solutions/Forge_Command`
- Default branch: `main`
- Inspected head: `fbcf51e1575e4d57152cfa7079f888d191bc7939`
- Head date: 2026-08-11
- Current implemented scope relevant to this plan: operator control surface, local APIs, evidence verification, audit state, run lifecycle, DataForge integration, and Playwright failure screenshot/trace configuration.
- Known gap: no verified `/ingest/tarcie` receiver or Tarcie-specific Beta Evidence Inbox was found.

## Source facts requiring Phase 0 verification

- Current build/test status at the pinned commits.
- Whether Tarcie's queue can duplicate already delivered batches after later-batch failure.
- Actual platform permission behavior for active-window screenshot capture.
- Exact Forge_Command API surface that should own intake.
- Exact DataForge Local artifact and receipt persistence API.
- Current canonical schema and receipt libraries to reuse.
- Required plan-registry file and folder metadata paths in the implementation repositories.

## Protocol basis

- BDS Evidence Receipt Protocol.
- BDS Receipt Spine Plan Set Revision A.
- Current Forge plan lifecycle, Board Review, machine-readable manifest, folder metadata, and registry conventions.
- Forge_Command evidence doctrine: immutable minimized evidence, exact identity, deterministic evaluation, and fail-closed conflict handling.

## Evidence limitation

Repository inspection supports the architecture decision but does not authorize implementation. Live runtime behavior, platform permissions, and actual DataForge/Forge_Command interoperability remain unproven.

