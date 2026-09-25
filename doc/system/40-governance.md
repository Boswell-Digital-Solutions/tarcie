# Governance

**Truth class:** canonical doctrine

This documentation system governs Tarcie's repo-local implementation truth. It
does not define ecosystem-level doctrine, DataForge truth ownership, or SMITH
downstream analysis behavior beyond the contract surfaces Tarcie consumes or
hands off to.

## Authority Boundary

- `doc/system/` is the canonical authored source tree for Tarcie system truth.
- `doc/TARSYSTEM.md` is generated output and must not be edited by hand.
- Supporting docs, plans, and archives outside `doc/system/` are subordinate to
  the compiled system reference when they describe current behavior.
- Runtime behavior and verification evidence override stale prose; when they
  disagree, update the source chapter and rebuild the compiled artifact.

## Change Control

Changes that alter capture behavior, queue durability, flush semantics,
configuration, sink contracts, or safety constraints must update the relevant
`doc/system/` chapter in the same change as the implementation.

Documentation-only changes must still rebuild `doc/TARSYSTEM.md` with:

```bash
bash doc/system/BUILD.sh
```
