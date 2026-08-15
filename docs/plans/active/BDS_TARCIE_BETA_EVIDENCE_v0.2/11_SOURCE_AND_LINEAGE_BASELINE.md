# Source and Lineage Baseline

## Lineage

`BDS-TARCIE-BETA-EVIDENCE-v0.1` remains the controlling reviewed plan until this
v0.2 candidate receives a new Board Review decision. The v0.1 BR1 decision dated
2026-08-14 authorized only Phase 0 source lock and Phase 1 documentation,
contract, fixture, threat-model, and ownership work with fail-closed conditions.
It authorized no runtime implementation.

This v0.2 candidate is required because the following are semantic changes:

- preloaded product profiles and tester assignments;
- a portable `.tarcie-session` package;
- a separate Session Hub;
- Start, Pause/Resume, End & Review, and Finalize lifecycle;
- section coverage tracking;
- displayed writable PDF closeout and structured round-trip;
- binding application support requirement for Windows, macOS, and Linux;
- explicit Author_Forge first proving target across installed platforms.

No existing BR1 authority is silently inherited for those changes.

## Connector-observed repository heads

| Repository | Branch | Observed commit | Observation time basis |
| --- | --- | --- | --- |
| `Boswell-Digital-Solutions/tarcie` | `main` | `7ad58c601312baaf9880bc3120a47e1b77bb34ed` | latest commit returned 2026-08-15 |
| `Boswell-Digital-Solutions/Author-Forge` | `main` | `5b7765113ead1465f4d8a3802abfc7e34e8a9b07` | latest commit returned 2026-08-13 |
| `Boswell-Digital-Solutions/Forge_Command` | `main` | `87fd7f11ad088079949f15a17752c93c29782fdb` | latest commit returned 2026-08-14 |
| `Boswell-Digital-Solutions/dataforge-Local` | `master` | `6c07d13d31a6f6c07789cc23429d1b474ec0816a` | latest commit returned 2026-08-14 |

These pins identify inspected source only. They are not clean-build receipts or
implementation authority.

## Tarcie current-state observations

- Tauri v2, Rust, and TypeScript desktop capture tool.
- 480 x 140 hidden, always-on-top overlay with input, marker, and status.
- Notes, markers, tags, crash-resistant JSONL queue, background sink flush.
- Current product contract is write-only and explicitly has no readback.
- Three IPC commands: capture note, capture marker, flush now.
- Current bundle targets are `.deb` and AppImage.
- Latest merged PR #20 reports 106 Rust and 29 frontend tests, archive bounds,
  and owner-only Unix paths. Independent GATE-00 reproduction remains required.
- No session package, Session Hub, product sections, writable PDF, Windows
  installer, or macOS installer exists in the observed surface.

## Author_Forge current-state observations relevant to this plan

- Tauri desktop architecture with native release workflow jobs for Linux
  x86_64, macOS Apple Silicon, and Windows x86_64.
- Build workflow includes test gate, private-dependency GitHub App gate,
  sidecars, Tauri action, and optional signing/notarization inputs.
- Build configuration is evidence of intent and automation, not installed
  support proof.
- Windows Scrivener 3 / Project Version 2 UUID qualification is an integration
  boundary and does not narrow the Author_Forge application platform contract.

## Claim limits

- No platform support claim is made by this plan set.
- No screenshot, PDF runtime, package loader, receiver, or persistence behavior
  is claimed as implemented.
- No CI result is claimed independently reproduced.
- No production DataForge contract is claimed located or approved.
- No live tester, personal content, real manuscript, or production application
  was used in preparing this packet.

