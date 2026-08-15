# CI, Evidence, and Acceptance Plan

## CI lanes

### Static contract lane

- validate all JSON/YAML and strict schemas;
- reject unknown fields in closed objects;
- verify examples against schemas;
- verify canonical JSON vectors and SHA-256 values;
- verify CHECKSUMS and evidence-index membership;
- scan session fixtures and PDFs for prohibited active content.

### Tarcie unit/integration lane

- package parser and path normalization;
- state machine and idempotency;
- section coverage rules;
- original/correction separation;
- spool durability and retention;
- action-registry equivalence across buttons, tags, and focused shortcuts;
- PDF field binding and finalization manifest;
- receiver retry and receipt verification.

### Cross-platform build lane

Native Windows, macOS, and Linux runners build platform installers. Builds are
gated on tests and pinned dependencies. Missing signing/notarization materials
must produce a clearly beta/unsigned artifact or fail according to release
policy; they may not create a false signed-release claim.

### Installed smoke lane

On each platform:

1. clean install;
2. first launch;
3. load canonical synthetic `.tarcie-session`;
4. Start, Pause, Resume, End & Review;
5. edit/save/reopen writable PDF;
6. Finalize and verify hashes;
7. restart recovery;
8. update and uninstall.

### Fault-injection lane

Disk full, permission denial, process kill, corrupt archive, unknown schema,
oversize attachment, path traversal, duplicate ID, hash conflict, screenshot
denial, PDF field loss, receiver down, timeout, partial acceptance, stale
receipt, and store failure.

## Evidence package

Every gate emits:

- source and dependency pins;
- environment/platform fingerprint;
- commands and exit status;
- test inventory and results;
- artifact/installer hashes;
- expected/actual comparison;
- failures and nonconformities;
- reviewer and decision status;
- receipt and supersession links.

Evidence is immutable. A rerun creates a new run ID and references the earlier
run rather than rewriting it.

## Acceptance metrics

- zero lost or duplicate observations in fault corpus;
- zero accepted items without verified receipts;
- 100% package/schema/hash conflict refusal;
- 100% required PDF fields present after save/reopen;
- zero PDF JavaScript, launch actions, or embedded files;
- zero sensitive content in logs;
- sub-five-second quick-note confirmation remains the current hard ceiling;
- 100% core workflow completion on each supported platform;
- zero unsupported-platform silent fallback;
- tester can identify product, build, current section, local/delivery state, and
  whether Finalize has occurred.

## Board Review 2 evidence

BR2 requires installed qualification receipts on all three platforms, limited
human beta sessions using disposable data, privacy/accessibility review,
recovery evidence, PDF comprehension results, receipt-chain proof, all open
nonconformities, and an explicit release recommendation.

