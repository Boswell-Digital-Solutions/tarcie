# 10. Testing

## Current State

Tarcie v1.0.0 has minimal automated tests. This is proportional to its scope: a 636 LOC write-only capture tool with 3 commands and no complex business logic.

## Building

```bash
cd src-tauri
cargo build
```

## Running in Development

```bash
cd src-tauri
cargo tauri dev
```

This launches the Tauri development server with hot-reload for the frontend.

## Running Tests

```bash
cd src-tauri
cargo test
```

## Type Checking

```bash
cd src-tauri
cargo check
```

## Lint

```bash
cd src-tauri
cargo clippy -- -W clippy::all
```

## What To Test (If Expanding)

If tests are added in the future, priority areas:

1. **Queue append + read round-trip** -- serialize, append, read back, verify
2. **Tolerant read** -- inject a malformed line, verify it is skipped and valid events are returned
3. **Cap rotation** -- append 10,001 events, verify rotation occurs
4. **Content clamping** -- verify content over 10 KB is clamped, not rejected
5. **Tag extraction** -- verify `#tag` regex extracts correctly, respects `MAX_TAG_CHARS`
6. **Sink URL validation** -- verify localhost-only enforcement when `TARCIE_ALLOW_REMOTE_SINK=false`
7. **FlushResult variants** -- verify Empty, Success, and Deferred paths
