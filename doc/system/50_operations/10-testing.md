# 10. Testing

## Current State

Tarcie has a Rust unit test suite and a frontend unit test suite.

The Rust tests live beside the code they cover, in `#[cfg(test)]` modules in
`queue/jsonl.rs`, `ipc/commands.rs`, `sink/config.rs`, `flusher.rs`,
`util/device.rs`, and `main.rs`.

The frontend tests live in `src/capture.test.ts` and run under Vitest. They
cover `src/capture.ts`, which holds the capture flow apart from the DOM and
from Tauri: the five-second revert of constraint 1, and the rule that only a
confirmed capture clears the box.

The suite covers the seven priority areas:

| Area | Module | Proves |
|------|--------|--------|
| Append and read round-trip | `queue/jsonl.rs` | An appended event reads back with its fields intact, and order holds |
| Tolerant read | `queue/jsonl.rs` | A malformed, truncated, or blank line is skipped; valid events still return |
| Cap rotation | `queue/jsonl.rs` | An append at the cap rotates the file first, and every capped event still reaches a claim |
| Content clamping | `ipc/commands.rs` | Oversized content is clamped, not rejected, and stays valid UTF-8 |
| Tag extraction | `ipc/commands.rs` | `#tag` becomes the context; absent tags fall back to the default |
| Sink URL validation | `sink/config.rs` | A remote sink is refused unless the operator opts in |
| FlushResult variants | `flusher.rs` | `Empty`, `Success`, and `Deferred` each occur, and a deferral keeps every event |

The suite also covers these areas:

| Area | Module | Proves |
|------|--------|--------|
| The command layer | `ipc/commands.rs` | A capture passes through clamp, tag, build, and append, and the queue holds the event the user meant |
| Device identity | `util/device.rs` | The ID is minted once, read back on every run after, and replaced when the file is damaged |
| Name reuse after a crash | `queue/jsonl.rs` | A name an earlier run left behind is never taken over, and an exhausted search fails instead of overwriting |
| The hotkey binding | `main.rs` | The documented `HOTKEY` string parses, and it names the combination that gets registered |
| The capture revert | `src/capture.ts` | A capture that outlives its budget reverts, a slow one inside the budget still counts, and a late reply is ignored |
| Overlay honesty | `src/capture.ts` | Only a confirmed capture flashes, hides the overlay, and clears the box |

Each command needs a Tauri `State`, which a test cannot supply. Each one
therefore delegates to a function over a plain `&AppState` — `capture_note_into`,
`capture_marker_into`, `flush_now_on` — and the test drives that function. The
command itself adds only the state extraction.

`load_or_create_device_id` resolves the identity file from the platform data
dir. `load_or_create_device_id_at` takes the path directly, so a test writes to
a temporary directory instead of the real user profile. `JsonlQueue::new_in`
gives the queue the same seam.

`free_path` takes the name generator as an argument. A test can therefore offer
a name that is already on disk, which is what a restarted sequence does, and
hold the guard to leaving that file alone.

`runCapture` takes the send and the budget as arguments, so a test drives both
with a fake clock and never waits five real seconds. `src/main.ts` keeps only
the DOM wiring, which no test covers.

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
`npm run check` and `npm test`, runs `cargo test`, and then runs
`bash doc/system/BUILD.sh`.

`npm run check` is `tsc --noEmit`. Vite builds with esbuild, which strips the
types without checking them, so nothing enforced the strict settings in
`tsconfig.json` before this step existed.

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
cargo test        # the Rust suite
```

```bash
npm test          # the frontend suite
npm run check     # tsc --noEmit
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

These areas have no tests:

1. **The mid-flush capture window at the flusher level.** The window itself is
   covered in `queue::jsonl`, where an append can be placed between the claim
   and the completion. A flusher test cannot reach inside `flush_with_retry`
   to do that, so no end-to-end version exists.
2. **A crash between a partial delivery and its archive.** The duplicate this
   produces is documented, not tested; it needs process-level fault injection.
3. **That every stamped path goes through `free_path`.** The guard itself has
   tests. That each of the four callers uses it — claim, defer, archive, and
   cap rotation — is verified by reading. Forcing a collision through a caller
   needs control of the clock and the sequence, which no seam offers.
4. **Hotkey registration, the window toggle, and the shutdown flush.** All
   three run on the event loop of a real window, so they need a running
   desktop session. The hotkey *string* is covered: a test proves `HOTKEY`
   parses and names the combination the code registers. Whether the operating
   system then grants that combination is not covered.
5. **The DOM wiring in `src/main.ts`.** The decisions it acts on live in
   `src/capture.ts` and have tests. That the key handler, the marker button,
   and the flash are wired to them is verified by reading. Covering the wiring
   needs a DOM in the test run, which the suite does not carry.
