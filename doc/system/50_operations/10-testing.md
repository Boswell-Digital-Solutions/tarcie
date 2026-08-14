# 10. Testing

## Current State

Tarcie has a unit test suite. The tests live beside the code they cover, in
`#[cfg(test)]` modules in `queue/jsonl.rs`, `ipc/commands.rs`, `sink/config.rs`,
and `flusher.rs`.

The suite covers the seven priority areas:

| Area | Module | Proves |
|------|--------|--------|
| Append and read round-trip | `queue/jsonl.rs` | An appended event reads back with its fields intact, and order holds |
| Tolerant read | `queue/jsonl.rs` | A malformed, truncated, or blank line is skipped; valid events still return |
| Cap rotation | `queue/jsonl.rs` | An append at the cap rotates the file first, and keeps every capped event |
| Content clamping | `ipc/commands.rs` | Oversized content is clamped, not rejected, and stays valid UTF-8 |
| Tag extraction | `ipc/commands.rs` | `#tag` becomes the context; absent tags fall back to the default |
| Sink URL validation | `sink/config.rs` | A remote sink is refused unless the operator opts in |
| FlushResult variants | `flusher.rs` | `Empty`, `Success`, and `Deferred` each occur, and a deferral keeps every event |

Every test asserts intended behavior. No test currently pins a known
deviation.

If a future test must record behavior that differs from the documented intent,
mark it with a `KNOWN DEVIATION` comment that states the deviation. A fix must
change that test in the same commit.

## Prerequisites

`cargo test` builds the full Tauri binary, so it needs the frontend bundle and
the Linux system libraries.

```bash
npm install && npm run build   # creates dist/, required by frontendDist
```

Without `dist/`, the build fails in `tauri::generate_context!`.

On Debian or Ubuntu, the system libraries are:

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev \
  build-essential pkg-config
```

## Continuous Integration

`.github/workflows/ci.yml` runs on every pull request and on a push to `main`.
It installs the system libraries, runs `npm ci` and `npm run build`, runs
`cargo test`, and then runs `bash doc/system/BUILD.sh`.

The final step runs `git diff --exit-code doc/TARSYSTEM.md`. A change under
`doc/system/` that ships without a rebuild fails the job.

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

## Not Yet Covered

The seven priority areas are covered. These areas are not:

1. **The Tauri command layer.** `capture_note`, `capture_marker`, and
   `flush_now` need a Tauri `State`, so the tests cover the logic they call
   instead of the commands themselves.
2. **The mid-flush capture window at the flusher level.** The window itself is
   covered in `queue::jsonl`, where an append can be placed between the claim
   and the completion. A flusher test cannot reach inside `flush_with_retry`
   to do that, so no end-to-end version exists.
3. **A crash between a partial delivery and its archive.** The duplicate this
   produces is documented, not tested; it needs process-level fault injection.
4. **The global hotkey and window toggle.** These need a running desktop
   session.
5. **Device ID persistence.** `load_or_create_device_id` writes to the real
   user profile and has no path seam.
