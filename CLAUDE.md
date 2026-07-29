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

No CI workflow and no test script exist in this repo. Build surfaces are `npm run build` and
`npm run tauri`; `bash doc/system/BUILD.sh` must still succeed after documentation changes.

```bash
./scripts/context-bundle.sh --list
```
