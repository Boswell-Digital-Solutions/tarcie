# 10. Testing

## Current State

Tarcie has a Rust unit test suite and a frontend unit test suite.

The Rust tests live beside the code they cover, in `#[cfg(test)]` modules in
`queue/jsonl.rs`, `ipc/commands.rs`, `sink/config.rs`, `sink/client.rs`,
`flusher.rs`, `util/device.rs`, `util/log.rs`, and `main.rs`.

The frontend tests run under Vitest and live beside the code they cover:

- `src/capture.test.ts` covers `src/capture.ts`, which holds the capture flow
  apart from the DOM and from Tauri — the five-second revert of constraint 1,
  and the rule that only a confirmed capture clears the box. It runs in the
  `node` environment, which is also a check that `capture.ts` needs no DOM.
- `src/overlay.test.ts` covers `src/overlay.ts`, the wiring from a keyboard, a
  button, and a window to those decisions. It declares
  `// @vitest-environment jsdom` at the top of the file.

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
| The log | `util/log.rs` | A report reaches the file stamped, the file rotates at its ceiling and keeps one previous, an over-long line is shortened, and a log that cannot be written does not take the caller down |
| The request bound | `sink/client.rs` | A sink that never answers is given up on at the bound, and a sink that answers inside the bound is not cut off |
| A sink that stops answering | `flusher.rs` | The production path carries a deadline, so a flush over a silent sink ends instead of running on |
| The deferral reason | `flusher.rs` | A deferral names the cause and not only the attempt |
| Nothing worth sending | `ipc/commands.rs` | A note that says nothing of its own never reaches the queue, whether it is empty, whitespace, a tag alone, or a string of tags, and a tagged observation still does |
| One capture per gesture | `src/overlay.ts` | An empty box sends nothing, a repeated Enter sends one capture, a refused note keeps its text on screen, and the next note is still taken |
| Durable placement | `queue/jsonl.rs` | A placement moves the file and neither directory sync errors, and a placement that cannot happen leaves the batch where it was |
| The capture revert | `src/capture.ts` | A capture that outlives its budget reverts, a slow one inside the budget still counts, and a late reply is ignored |
| Overlay honesty | `src/capture.ts` | Only a confirmed capture flashes and hides the overlay, and only a confirmed note clears the box |
| The overlay wiring | `src/overlay.ts` | Enter sends what was on screen, Escape puts the overlay away without capturing, the marker button captures with no reason, and the overlay arrives focused |
| Text the user has not lost | `src/overlay.ts` | A refusal and a timeout both leave the box and the window alone, and a reply arriving after the revert never takes text typed since |

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
with a fake clock and never waits five real seconds.

`wireOverlay` takes its ports as an argument: the four elements, the invoke
call, and the window's hide. `src/main.ts` supplies the real ones; a test
supplies a document it built and calls it can drive. `src/main.ts` is then
bootstrap only.

`SinkClient::with_timeout` takes the request bound directly, so a test proves
it in a quarter of a second rather than waiting out `SINK_REQUEST_TIMEOUT_SECS`.
It is `cfg(test)`, because it also accepts no bound at all and production must
not be able to ask for that.

The bound is proven on a real clock, which is deliberate. A paused clock
advances to the nearest deadline as soon as the runtime idles, and waiting on a
real socket idles it. A paused test therefore cannot tell a sink that never
answers from one that answers at once, because both end at the bound. Two
real-clock tests hold it instead: a silent sink is given up on, and a healthy
sink is not cut off. The second is what keeps the first from passing against a
client that refuses everything.

One paused test needs a real exchange to succeed —
`a_partial_multi_batch_delivery_retries_only_what_is_owed`, where the sink must
accept the first batch. It runs through `flusher_unbounded`, with no bound for
the paused clock to jump to. The paused test beside it holds the opposite
ground: with no bound in `SinkClient::new` there is no deadline at all, the
flush never returns, and its guard reports that rather than hanging the suite.

`LogFile::with_ceiling` takes the size ceiling directly, so a test proves the
rotation without writing a megabyte to do it. The tests build a `LogFile` in a
temporary directory and never call `log::init_in`, so the process-wide log stays
closed and nothing in the suite writes to the real user profile.

Every test asserts intended behavior. No test currently pins a known deviation.

One stood briefly: a marker cleared a note the user typed and never captured,
because a confirmed capture of any kind cleared the box. Clearing now belongs
to the note capture alone, and the test that recorded the deviation asserts the
fix.

A test that must record behavior differing from the documented intent is marked
with a `KNOWN DEVIATION` comment that states the deviation. A fix must change
that test in the same commit.

## Prerequisites

**Node 22.22.2 or later**, as `package.json` declares. jsdom 30 needs it: on
Node 20 its undici dependency reaches for `webidl` internals that are not there
yet, and the overlay suite dies on import rather than failing a test. CI pins
the same version.

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
   needs control of the clock and the sequence, which no seam offers. The same
   reading covers `rename_durably`, which the same four callers use.
4. **That a placement survives a power loss.** `rename_durably` and the append
   that creates the queue file both sync the directory, and the tests prove the
   syncs run and cost nothing in behaviour. Whether the entry is on the platter
   afterwards is a property of the disk, and proving it needs power-loss
   injection rather than a unit test.
5. **Hotkey registration, the window toggle, and the shutdown flush.** All
   three run on the event loop of a real window, so they need a running
   desktop session. The hotkey *string* is covered: a test proves `HOTKEY`
   parses and names the combination the code registers. Whether the operating
   system then grants that combination is not covered.
6. **The bootstrap in `src/main.ts`.** It finds the four elements and hands
   `wireOverlay` the real Tauri calls, and does nothing else. Reaching it needs
   a running webview, because `@tauri-apps/api` only resolves inside one. The
   wiring it hands over is covered in `src/overlay.test.ts`.
