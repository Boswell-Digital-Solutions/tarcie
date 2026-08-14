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

Some tests pin behavior that deviates from the documented intent. Each one
carries a `KNOWN DEVIATION` comment that states the deviation. These tests
record what the code does today. They do not endorse it. A future fix must
change the test in the same commit.

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
2. **Concurrent append during a flush.** The flusher reads, posts, and then
   rotates. An append between the read and the rotate is filed as sent. No
   test covers this window yet.
3. **Multi-batch flush.** A flush that succeeds on one batch and fails on a
   later one re-sends the succeeded batch on the next cycle.
4. **Rotation timestamp collisions.** Rotation names files to the second. Two
   rotations in one second overwrite each other.
5. **The global hotkey and window toggle.** These need a running desktop
   session.
6. **Device ID persistence.** `load_or_create_device_id` writes to the real
   user profile and has no path seam.
