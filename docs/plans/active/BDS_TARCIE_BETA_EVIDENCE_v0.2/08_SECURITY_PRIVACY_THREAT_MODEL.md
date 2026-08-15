# Security and Privacy Threat Model

## Assets

- session package and assignment identity;
- tester-authored observations and drafts;
- screenshots and redacted derivatives;
- product/build/platform provenance;
- writable PDF fields and structured report;
- finalization manifest, hashes, and receiver receipts;
- local encryption keys and delivery credentials when later authorized.

## Trust boundaries

1. Forge_Command package authoring to portable session file.
2. Portable file to Tarcie package loader.
3. Tarcie webview to backend-owned action/session engine.
4. Platform screenshot/permission surface to Tarcie preview.
5. Tarcie report workspace to PDF component.
6. Local spool to receiver.
7. Receiver to DataForge Local persistence.

## Threats and controls

### Malicious or malformed session package

Threats: traversal, zip bomb, executable content, schema confusion, duplicate
IDs, hash substitution, symlinks, credential injection, malicious links.

Controls: normalized relative paths, no links, strict schemas, admitted media
types, compressed/uncompressed limits, depth/count limits, per-file and package
hashes, duplicate detection, no auto-open, and all-or-nothing admission.

### Renderer authority escalation

Threats: renderer invokes arbitrary backend commands or forges consent.

Controls: typed action IDs, backend-owned registry, focused user activation,
state transition validation, one active transaction, no arbitrary paths or
commands from the renderer, and restrictive CSP before any readback surface.

### Wrong-window or overbroad capture

Threats: silent screen capture, widened fallback, multiple displays, permission
reuse mistaken for consent.

Controls: foreground action, platform-visible selection, exact qualified scope,
preview, explicit Save, no scope widening, and `BLOCKED` when unavailable.

### Sensitive-content persistence

Threats: raw pixels, drafts, private messages, terminals, credentials, or real
manuscripts survive cancellation or appear in logs.

Controls: denylist/allowlist profiles, ephemeral encrypted source material,
flattened re-encoded derivative only, verified cleanup, owner/app boundary,
encrypted drafts, content-free logs, retention caps, and cleanup receipts.

### Writable PDF attack surface

Threats: PDF JavaScript, launch actions, embedded files, parser vulnerabilities,
duplicate/orphaned fields, stale appearances, post-finalization edits.

Controls: trusted generated template only, prohibited active features, pinned
renderer, sandbox/capability restrictions, canonical field map, AcroForm and
widget validation, save/reopen proof, final byte hash, and explicit revision
instead of silent overwrite.

### Replay and false acceptance

Threats: duplicate submission, same ID/different content, forged 2xx, partial
failure, stale receipt.

Controls: idempotency key, canonical hashes, per-item receipts, conflict refusal,
receiver identity/version, durable-store reference, deterministic retry, and no
accepted UI state without receipt verification.

### Platform divergence

Threats: a safe path on one OS silently becomes an unsafe fallback on another.

Controls: platform capability matrix, no inference across platforms, installed
qualification, identical corpus, platform-specific denial tests, and explicit
unsupported states.

## Logging policy

Logs may contain IDs, versions, hashes, sizes, state transitions, reason codes,
and bounded timing. Logs may not contain observation text, drafts, PDF narrative
values, screenshots, thumbnails, filenames, window titles, source paths,
credentials, tokens, or manuscript content.

## First proving data policy

Use synthetic or disposable Author_Forge data only. No real manuscript,
production account, private communications, or credential-bearing surface is
admitted. Network transmission and production storage remain disabled.

## Security stop conditions

Any path escape, active PDF feature, unreviewed capture, unverifiable cleanup,
unencrypted sensitive persistence, secret in logs, duplicate acceptance,
silent fallback, cross-platform claim without proof, or unbounded storage stops
the work package immediately.

