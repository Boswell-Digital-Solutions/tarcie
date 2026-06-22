# tarcie - Compiled System Reference

**Designation:** TAR
**Document role:** Canonical compiled technical reference for the Tarcie local capture tool
**Source:** `doc/system/`
**Build command:** `bash doc/system/BUILD.sh`
**Document version:** 2.0 (2026-06-22) - canonical compliance migration
**Protocol:** BDS Documentation Protocol v2.0; BDS Repo Documentation System Canonical Compliance Standard

> **Generated artifact warning:** `doc/TARSYSTEM.md` is assembled output. Edit
> the source modules under `doc/system/` and rebuild. Hand edits to the
> compiled artifact are overwritten by the next build.

Assembly contract:

- Command: `bash doc/system/BUILD.sh`
- Validation: `bash doc/system/validate_snapshots.sh` runs during assembly
- Primary output: `doc/TARSYSTEM.md`

This `doc/system/` tree is the canonical source of truth for tarcie. It
uses explicit **truth classes**: canonical facts define the repo role, authority
boundaries, runtime behavior, service contracts, and verification doctrine;
snapshot facts are dated, audit-derived counts and current implementation
inventory that may drift between audits.

| Part | File | Contents |
| --- | --- | --- |
| §1 | `00_overview/01-overview.md` | 1. Overview |
| §2 | `00_overview/02-architecture.md` | 2. Architecture |
| §3 | `10_service-contract/03-command-reference.md` | 3. Command Reference |
| §4 | `10_service-contract/10-product-surface.md` | Product Surface |
| §5 | `20_runtime/04-data-model.md` | 4. Data Model |
| §6 | `20_runtime/05-queue-system.md` | 5. Queue System |
| §7 | `20_runtime/06-flush-pipeline.md` | 6. Flush Pipeline |
| §8 | `20_runtime/09-error-handling.md` | 9. Error Handling |
| §9 | `20_runtime/20-runtime.md` | Runtime |
| §10 | `30_dependencies/40-integrations.md` | Integrations |
| §11 | `50_operations/07-configuration.md` | 7. Configuration |
| §12 | `50_operations/08-constraints.md` | 8. Constraints |
| §13 | `50_operations/10-testing.md` | 10. Testing |
| §14 | `50_operations/11-handover.md` | 11. Handover |
| §15 | `50_operations/50-operations.md` | Operations |
| §16 | `99_appendices/30-data.md` | 4. Data Model |
| §17 | `99_appendices/90-appendices.md` | Appendices |
| §18 | `99_appendices/91-bootstrap-overview.md` | 1. Overview |
| §19 | `99_appendices/92-bootstrap-architecture.md` | 2. Architecture |

## Quick Assembly

```bash
bash doc/system/BUILD.sh
```
