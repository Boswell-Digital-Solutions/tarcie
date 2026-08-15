# Cross-Platform Qualification Plan

## Product-level requirement

Tarcie and Author_Forge must both be installable and operational on Windows,
macOS, and Linux. This is an application requirement. Individual integrations
may carry narrower claims only when that limitation is explicit and does not
misrepresent the application itself.

The Windows-only Scrivener 3 Project Version 2 UUID qualification remains an
integration claim. It is not an Author_Forge platform claim.

## Qualification rule

No platform is called supported merely because source compiles or CI emits an
artifact. Support requires an installed-system receipt for the defined core
workflow.

## Baseline platform matrix

| Platform | Minimum candidate packaging | Required core proof |
| --- | --- | --- |
| Windows | Signed or explicitly beta-marked installer appropriate to the approved architecture | Install, launch, package load, session lifecycle, capture, report, PDF save/reopen, restart recovery, update, uninstall |
| macOS | Signed/notarized or explicitly beta-marked app package for the approved architecture | Same core proof plus permission prompts, focus, sandbox/path and PDF-viewer behavior |
| Linux | `.deb` and/or AppImage for the approved distribution baseline | Same core proof plus desktop environment, Wayland/X11, portal, permissions, and package integration |

Minimum OS versions and architecture coverage remain GATE-00 decisions. An OS
name never silently implies every architecture or distribution.

## Tarcie-specific proof

- Installer launches an accessible Session Hub and registers only admitted file
  associations/shortcuts.
- `.tarcie-session` packages load identically and hash to the same canonical
  structured payload.
- Global capture hotkey behavior, conflicts, focus, always-on-top, and recovery
  are tested. Unsupported registration must be visible, not silent.
- Screenshot capture uses the qualified native/portal path for that platform,
  never a scope-widening fallback.
- App-owned storage permissions and encryption are proven using the platform's
  actual security model; Unix mode bits are not treated as Windows protection.
- Writable PDF fields save, reopen, render, and round-trip on the qualified PDF
  component/viewer.

## Author_Forge-specific proof

- The exact tested Author_Forge build installs and launches on the platform.
- Required local services start or fail visibly according to their documented
  required/optional classification.
- The synthetic/disposable Author_Forge project opens without production or
  personal manuscript content.
- Assigned product sections are reachable through real user surfaces.
- Platform-specific file pickers, exports, sidecars, permissions, and shutdown
  behavior are recorded.
- A narrower feature qualification, such as Windows Scrivener import, remains
  visibly narrower in the report.

## Cross-platform equivalence corpus

One canonical synthetic session package runs on all three platforms. The
normalized structured outputs must agree on:

- product/build/assignment identity;
- section order and final coverage states;
- observation ordering and canonical hashes;
- report field values and field-map version;
- finalization manifest and receipt semantics.

Platform fingerprints, native paths, installer metadata, permission events,
and PDF byte layout may differ and are compared through admitted normalization
rules rather than ignored.

## Platform release gate

Each platform produces:

1. build provenance;
2. installer hash and signature/notarization state;
3. clean-install receipt;
4. core-workflow test report;
5. permission and privacy report;
6. PDF logical and visual validation;
7. update and uninstall receipt;
8. known limitations and unsupported integration claims.

Failure on one platform blocks that platform's support claim but does not erase
evidence for the others. A three-platform product release requires all three
platform gates to pass.

