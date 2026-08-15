# Session Package and UX Specification

## Preparation in Forge_Command

The operator selects a product profile, exact build, tester, platform policy,
assigned sections, prompts, time window, privacy profile, and optional free
exploration. Forge_Command previews the package exactly as Tarcie will show it,
then emits a hash-bound `.tarcie-session` file.

The operator sends the tester:

1. the appropriate Author_Forge installer;
2. the appropriate Tarcie installer; and
3. the assignment-specific `.tarcie-session` package.

The application is never rebuilt or secretly configured per tester.

## Tarcie surfaces

### Quick Capture Overlay

Preserves the small, always-ready interaction for notes, markers, and later
explicit screenshots. It shows the active product and section in compact text
but does not become a dashboard.

### Session Hub

A separate resizable window containing:

- product, build, platform, tester, and session identity;
- Start, Pause/Resume, End & Review controls;
- assigned section cards with required/optional status;
- current, remaining, blocked, skipped, and completed work;
- privacy and evidence guardrails;
- optional free-exploration entry;
- local/delivery state using text and icon redundancy;
- help and support reference supplied by the package.

### Report Workspace

Entered through End & Review. Displays the writable PDF report and a structured
field navigator. It supports Save Draft, Return to Session, Exclude Evidence,
Redact/Correct, Reopen Review, and Finalize Session.

## State machine

`NO_PACKAGE -> READY -> ACTIVE <-> PAUSED -> REVIEWING -> FINALIZED`

Additional terminal load states are `REJECTED`, `EXPIRED`, and `UNSUPPORTED`.
Delivery states are separate and cannot change session lifecycle state.

## Load Session

Tarcie validates the entire package before showing Start. A failure shows the
specific non-sensitive reason and preserves no partially admitted package.

The Ready screen displays:

- product and exact build;
- platform and assignment compatibility;
- assigned sections and estimated scope;
- what Tarcie may capture;
- what remains local;
- package producer, time, expiry, and hash;
- confirmation that no real manuscript or private production content is
  authorized in the first Author_Forge proving program.

## Start Session

Start requires explicit confirmation of the product/build identity and privacy
profile. It creates the durable local session state before the interface reports
Active. Failed durability leaves the session Ready.

The first assigned section becomes suggested, not forced. The tester can choose
another assigned section or optional free exploration. Tarcie records coverage,
not navigation authority over Author_Forge.

## Active session

Each section card shows state, purpose, prompts, captured items, missing required
items, and tester notes. The tester may mark Partial, Blocked, Skipped, or Not
Applicable only with a reason when required by the assignment.

Quick actions remain redundant:

- visible buttons;
- admitted leading `#action` tags; and
- focused shortcuts.

All three resolve through one typed action ID and one backend state machine.
Context tags such as `#bug` and `#idea` never execute.

## Pause and resume

Pause stores state durably, stops active-time accounting, and preserves the
current section and draft. Resume requires the same package/session identity.
Restart recovery must never create a second session.

## End & Review

The button label is **End & Review**, not simply End, because it enters a
reversible review state rather than finalizing immediately.

Before entering Review, Tarcie durably records the section states, selected
evidence, drafts, and pending exclusions. The writable report then opens with:

- product/build/platform/session facts pre-populated;
- section coverage and captured evidence references;
- expected/observed/reproducibility prompts for each finding;
- unresolved, untested, blocked, and skipped work;
- overall experience and priority concerns;
- privacy review and tester attestation.

The tester may return to Active until Finalize.

## Finalize Session

Finalize requires:

- all required sections resolved or explained;
- all selected evidence reviewed;
- all redaction/exclusion decisions complete;
- required report fields valid;
- writable PDF saved and logically re-read;
- PDF and structured report hashes computed;
- explicit tester approval.

Finalize produces a local candidate evidence package. It does not imply
delivery, receiver acceptance, issue creation, severity, or defect verdict.

## Accessibility

- All controls are keyboard and screen-reader operable.
- Focus order follows visual order and survives state changes.
- Section and delivery states have text labels, icons, and announcements.
- No workflow requires color, drag-and-drop, or canvas-only annotation.
- PDF fields have accessible names, multiline support, logical tab order, and
  a structured navigator outside the page canvas.
- Enlarged text and 200% zoom reflow without hiding actions.

