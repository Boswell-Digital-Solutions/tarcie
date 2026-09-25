# Product Surface

Everything a person can do with tarcie, and everything tarcie says back.

The surface is one window with one text box, one button, and four gestures.
There is nothing else: no menu, no tray icon, no settings screen, no history,
and no notifications.

## Entry

The global hotkey `Ctrl+Alt+T` is the only way in. It toggles the overlay:
visible becomes hidden, hidden becomes visible and focused.

The window is created hidden (`"visible": false`), skips the taskbar, and has no
tray icon, so nothing on screen offers a way to open it. A hotkey that the
operating system refuses to grant therefore leaves tarcie unreachable. Section
10 records that the binding is proven to parse and to name the combination the
code registers, and that whether the system grants it is not covered.

Repeated presses inside `HOTKEY_DEBOUNCE_MS` (500 ms) are ignored, so a key that
repeats does not flicker the window.

## The window

From `src-tauri/tauri.conf.json`:

| Property | Value | What it means on screen |
|---|---|---|
| `width` × `height` | 480 × 140 | Constraint 5: small enough not to take over |
| `resizable` | `false` | One size; there is nothing to lay out |
| `alwaysOnTop` | `true` | It sits over the work being observed |
| `skipTaskbar` | `true` | It does not appear as a running window |
| `visible` | `false` | It starts hidden and waits for the hotkey |
| `center` | `true` | It arrives in the same place every time |
| `decorations` | `true` | It keeps a title bar, and therefore a close button |
| `title` | `Tarcie` | — |

**The close button is not the Escape key.** Escape hides the overlay. Closing
the window ends the application, after a final flush bounded by
`SHUTDOWN_FLUSH_SECS`. Section 6 describes that flush.

## What is on it

Three elements, in `src/index.html`:

| Element | Id | Appearance |
|---|---|---|
| Text box | `tarcie-input` | Placeholder `Type one friction note… (optional #tag)`, focused on arrival |
| Marker button | `tarcie-marker` | A red circle, titled `Marker` |
| Status | `tarcie-status` | Empty except during a confirmation |

## The four gestures

| Gesture | What it does |
|---|---|
| `Ctrl+Alt+T` | Shows the overlay, focused, or hides it |
| `Enter` | Captures the text in the box as a note |
| Marker button | Captures a marker, labelled with whatever is in the box |
| `Escape` | Hides the overlay and captures nothing |

Section 3 states what each capture becomes, including which inputs are refused
and how a `#tag` is read.

`Escape` leaves the text where it is. The overlay is hidden rather than
destroyed, so an unsent draft is still in the box at the next hotkey press. It
survives until the application exits.

## What tarcie says back

One word, once, and only when a capture is confirmed.

The body takes a green outline, the status reads `Captured`, and both last
`FLASH_MS` (200 ms). The overlay then hides, and the box is cleared if that
capture took its text.

That is the whole vocabulary. There is no progress indicator, no error dialog,
no failure state, and no sink or queue status anywhere on screen.

**Silence is the other half of it.** A refused capture, a capture that outlived
its five-second budget, and a gesture turned away by a guard all look
identical: nothing changes. The overlay stays open, holding the text. The window
that did not go away is the whole signal, and the text still on screen is the
only copy anybody can point to. Section 9 sets this out.

## What is deliberately not here

- **No readback.** Nothing displays, searches, edits, or exports what was
  captured. Constraint 2 makes this a boundary rather than a gap: a "show me
  what I captured" surface is a scope change.
- **No settings screen.** Every setting is an environment variable, read once at
  startup. Section 7 lists them.
- **No status surface.** Whether delivery is working is reported to a log file,
  never to the overlay. Section 9 describes the log.
- **No account, no sync, no sharing.** Tarcie captures and forwards.

## The webview

The overlay renders no captured text as HTML. The box holds what the user
typed, the status holds one fixed word, and nothing from the queue or the sink
is ever displayed — which follows from there being no readback at all.

`"csp": null` in the window's security block disables the webview content
security policy. No injection path exists today, because no untrusted content
reaches the page. A surface that displays anything captured, or anything a sink
returns, would change that and needs the policy settled first.
