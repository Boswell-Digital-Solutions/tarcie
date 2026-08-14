# tarcie — Claude Code Context

Friction-free **write-only** capture tool: hotkey overlay → durable queue → HTTP sink. Tauri
desktop app.

Canonical reference: `doc/system/` → `doc/TARSYSTEM.md` (`bash doc/system/BUILD.sh`).
`doc/TARSYSTEM.md` is a build artifact; edit the parts, never the artifact.

---

## Boundaries

- **Write-only by design.** tarcie captures and forwards; it does not read back, browse, edit, or
  render the sink's contents. A "show me what I captured" feature is a scope change, not a
  refinement.
- The durable queue is the reliability contract: a capture survives the sink being unreachable.
  Never drop an item to keep the overlay responsive.
- Do not invent undocumented APIs, tables, routes, or environment variables.

---

## Verification

```bash
npm install && npm run build      # creates dist/ — cargo needs it
npm run check && npm test         # tsc --noEmit, then 9 frontend tests
cd src-tauri && cargo test        # 76 Rust unit tests
```

**`cargo test` fails without `dist/`.** `tauri::generate_context!` reads the
`frontendDist` path at compile time, so the Rust tests do not build until the
frontend bundle exists. The build also needs the Tauri Linux system libraries
(`libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, and the rest listed in
`doc/system/50_operations/10-testing.md`).

`.github/workflows/ci.yml` runs the same ground on every pull request. It
installs the system libraries, builds the frontend, runs the tests, and rebuilds
the system document. It also fails if `doc/TARSYSTEM.md` is stale, so a change
under `doc/system/` must ship with its rebuild.

No test currently pins a known deviation. If one must record behavior that
differs from the documented intent, mark it with a `KNOWN DEVIATION` comment
that states the deviation. A fix must change that test in the same commit.

```bash
./scripts/context-bundle.sh --list
```
