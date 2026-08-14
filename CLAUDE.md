# tarcie — Claude Code Context

Friction-free **write-only** capture tool: hotkey overlay → durable queue → HTTP sink. Tauri
desktop app.

Canonical reference: `doc/system/` → root `SYSTEM.md` (`bash doc/system/BUILD.sh`). `SYSTEM.md` is
a build artifact; edit the parts, never the artifact.

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
cd src-tauri && cargo test        # 43 unit tests
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

Some tests carry a `KNOWN DEVIATION` comment. These pin behavior that differs
from the documented intent. Do not treat them as endorsement — a fix must
change the test in the same commit.

```bash
./scripts/context-bundle.sh --list
```
