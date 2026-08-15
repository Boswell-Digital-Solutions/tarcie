# Build Reproduction — the test and document claims, reproduced

**Prepared:** 2026-08-15
**Status:** preparation input for Board Review 1 and GATE-00. Not a GATE-00
receipt, not a packet file, not a decision.

## What this is

The first independent reproduction of tarcie's build and test claims. Every
figure below was produced by running the commands in section 6 in a bounded
environment, not read from a pull request or returned by a connector.

Both plan sets ask for exactly this and neither has it:

- v0.1 Phase 0 — *"Reproduce actual Tarcie capture paths, queue behavior, and
  tests."*
- v0.2 `11_SOURCE_AND_LINEAGE_BASELINE.md` — *"Latest merged PR #20 reports 106
  Rust and 29 frontend tests […] Independent GATE-00 reproduction remains
  required."*
- v0.2 `02_MASTER_IMPLEMENTATION_PLAN.md` Phase 0 — *"replace connector
  observations with independently reproducible source and build evidence."*

## What this is not

This grants nothing and decides nothing. The v0.1 plan set is `proposed` and
`documentation-only`, the v0.2 candidate is awaiting Board Review 1, and the
supersession rule requires a new reviewed revision for any semantic change. So
this edits no reviewed document. It sits beside `00_SESSION_BRIEF.md` and
`01_RETENTION_GAP.md`, which is the layout the plan set already uses for gate
preparation.

Where this and a packet disagree, the packet is the document under review.

---

## 1. What was reproduced

Tarcie at `920c35d` — `main` and
`claude/beta-work-continuation-5y5xip` at the same commit — reproduced
2026-08-15.

| Claim | Source of the claim | Reproduced | Result |
|---|---|---|---|
| Rust unit tests | `CLAUDE.md` says 124 | **Yes** | **124 passed, 0 failed** |
| Frontend unit tests | `CLAUDE.md` says 29 | **Yes** | **29 passed, 2 files** |
| TypeScript typecheck | CI step | **Yes** | clean, no output |
| System document build | `CLAUDE.md` | **Yes** | `BUILD_OK designation=TAR parts=20 lines=1784` |
| Document not stale | CI step | **Yes** | `git diff --exit-code` clean |

Every claim held. **PR #20's figure of 106 Rust tests is stale rather than
wrong** — it was true when written, and the suite has grown since. `CLAUDE.md`
is current, and the v0.2 baseline should read 124 rather than 106 for this
commit.

### Test distribution

| File | Tests |
|---|---|
| `queue/jsonl.rs` | 40 |
| `ipc/commands.rs` | 31 |
| `sink/config.rs` | 18 |
| `schedule.rs` | 11 |
| `util/device.rs` | 8 |
| `flusher.rs` | 7 |
| `util/log.rs` | 6 |
| `sink/client.rs` | 2 |
| `main.rs` | 1 |
| **Total** | **124** |

## 2. Environment

The figures are only as good as the machine under them.

| Item | Value |
|---|---|
| OS | Ubuntu 24.04 (noble), Linux 6.18.5 x86_64 |
| Node | v22.22.2 |
| npm | 10.9.7 |
| cargo | 1.94.1 (2026-03-24) |
| rustc | 1.94.1 (2026-03-25) |
| Container | ephemeral, disk-backed, unloaded |

`libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`,
`librsvg2-dev`, `libsoup-3.0-dev`, `build-essential`, and `pkg-config` were
installed first. Without them the build stops at `gdk-sys`, before it reaches
any tarcie code — the same list `.github/workflows/ci.yml` installs.

`cargo test` does not build until `dist/` exists, because
`tauri::generate_context!` reads `frontendDist` at compile time. `npm run build`
comes first.

## 3. Capture latency — the Phase 0 item that was open

`00_SESSION_BRIEF.md` §4 records note latency as **"Not measured […] needs a
live desktop session."** That is right about the whole gesture and too strong
about the part that decides it.

The gesture divides in two:

1. **Hotkey to keystroke** — global shortcut, window show, focus, paint.
   Needs a desktop session. **Still not measured.**
2. **Keystroke to durable** — `capture_note` through `JsonlQueue::append`,
   ending at the `fsync` that makes the capture survive a power loss.
   **Headless, deterministic, and measured here.**

Part 2 is where the durability contract is kept, and it is the part that can
grow without anyone noticing, because it grows with the queue rather than with
the machine.

Measured over 50 appends at each depth, ordinary note content (~310 bytes):

| Queue depth | p50 | p95 | max |
|---|---|---|---|
| 0 | 367 µs | 606 µs | 23,373 µs |
| 1,000 | 936 µs | 1,142 µs | 1,553 µs |
| 2,500 | 1,552 µs | 1,713 µs | 1,769 µs |
| 5,000 | 2,658 µs | 2,837 µs | 2,982 µs |
| 7,500 | 3,793 µs | 4,095 µs | 4,308 µs |
| 9,999 | 5,312 µs | 5,574 µs | 7,360 µs |

Linear in the depth of the queue, and a factor of fourteen across the range the
default cap allows.

**The cause.** `append` checked the cap by opening `queue.jsonl` and counting
every line in it, before every write. One capture paid for a pass over the whole
backlog.

**Why it is the ordinary case.** §5 of `00_SESSION_BRIEF.md` records that the
default sink is `http://127.0.0.1:8080/ingest/tarcie` and that nothing serves
that port. An installation nobody has configured therefore never delivers,
accumulates from its first capture, and climbs this curve — while the overlay is
the surface the cost lands on.

**Corrected at this commit.** The count is now held beside the file, under the
mutex that already guarded it, and read from the file once when the queue is
built. The same measurement, same machine, same run:

| Queue depth | p50 | p95 | max |
|---|---|---|---|
| 0 | 286 µs | 451 µs | 15,787 µs |
| 1,000 | 275 µs | 364 µs | 386 µs |
| 2,500 | 309 µs | 462 µs | 512 µs |
| 5,000 | 308 µs | 474 µs | 539 µs |
| 7,500 | 347 µs | 446 µs | 475 µs |
| 9,999 | 479 µs | 552 µs | 582 µs |

Flat rather than linear. At the cap, p95 falls from 5,574 µs to 552 µs.

Both `max` figures at depth 0 are first-touch cost — directory creation and
allocator warm-up on the first append of the process — and appear at that depth
only.

### What this does and does not settle

- It **does** give GATE-01 a reproducible baseline for the durable half of the
  capture path, and a harness that produces it on demand.
- It **does not** set the threshold GATE-01 measures against. §10 item 4 of
  `00_SESSION_BRIEF.md` lists that as an operator decision and it is still open.
- It **does not** measure the desktop half. Constraint 1 in `constraints.rs`
  bounds the whole gesture at five seconds; against that, the durable half at
  the cap was consuming about a tenth of one percent of the budget before the
  correction and about a hundredth after. Neither figure says what the overlay
  costs.
- The harness is `#[ignore]`d. It records a shape, not a gate, and CI does not
  run it. Timings are machine- and filesystem-dependent and should be re-taken
  rather than compared across machines.

## 4. Source surface, confirmed at this commit

Re-checked directly, not inherited:

| Fact | State |
|---|---|
| IPC commands | Three: `capture_note`, `capture_marker`, `flush_now` |
| Bundle targets | `deb`, `appimage`. No Windows or macOS target |
| `csp` | `null` in `tauri.conf.json` — webview CSP disabled |
| Capabilities directory | Absent — `src-tauri/capabilities/` does not exist |
| Default sink | `http://127.0.0.1:8080/ingest/tarcie` |
| Remote sink | Refused unless `TARCIE_ALLOW_REMOTE_SINK=true` |
| Encryption at rest | None, on any of the four local surfaces |
| Local surfaces | `queue/`, `queue/sending/`, `queue/sent/`, `logs/`, `device_id.txt`, `last_scheduled_flush.txt` |
| Directory mode | `0o700` on Unix; `owner_only_file` opens at `0o600` |
| Readback | None. No IPC command, route, or UI surface reads a capture back |
| Event fields | `id`, `device_id`, `timestamp_utc`, `timestamp_mono_ms`, `event_type`, `content`, `app_context`, `source_version` |
| Event types | `Note`, `Marker { reason }` |
| Source version | `tarcie-v1.0.0` |

Nothing contradicts §7 of `00_SESSION_BRIEF.md`: this is still note and marker
capture, a JSONL queue, and a generic HTTP sink. **No beta-session identity,
artifact contract, screenshot path, session package, Session Hub, or PDF
surface exists.** Every v0.2 capability remains unimplemented, as its claim
limits state.

`"csp": null` is unchanged and remains the open decision §10 item 5 names,
ahead of any phase that renders content the user did not type.

## 5. Effect on the gate

- The v0.2 baseline's test figures can be replaced with reproduced ones for
  this commit. The direction of the correction is upward: the suite is larger
  than the pinned observation said, not smaller.
- One Phase 0 checklist row moves from *"Not answerable here"* to **answered for
  the durable half, still open for the desktop half**.
- The capture-latency threshold, the encryption-at-rest decision, and the
  `"csp": null` decision are untouched. They are operator decisions and this
  settles none of them.

## 6. Reproducing this

```bash
# Ubuntu 24.04, Node 22.22.2 or later, from the repo root
sudo apt-get update && sudo apt-get install -y \
    libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
    librsvg2-dev libsoup-3.0-dev build-essential pkg-config

npm ci && npm run build          # dist/ must exist before cargo
npm run check                    # tsc --noEmit
npm test                         # expect 29 passed
cargo test --manifest-path src-tauri/Cargo.toml   # expect 126 passed
bash doc/system/BUILD.sh && git diff --exit-code doc/TARSYSTEM.md

# The latency shape in section 3. Not a gate; run it deliberately.
cargo test --manifest-path src-tauri/Cargo.toml \
    what_a_capture_costs_as_the_queue_fills -- --ignored --nocapture
```

The Rust figure is 124 at `920c35d` and 126 after the correction in section 3,
which adds two tests: one holding that a restart still counts what an earlier
run queued, and one holding that a claim empties the count along with the queue.

No sibling repository was read or modified in preparing this. Sections 1 to 4
concern tarcie alone; the cross-repository rows of the source lock are
unchanged from `00_SESSION_BRIEF.md` §§1, 5, 6, and 7.
