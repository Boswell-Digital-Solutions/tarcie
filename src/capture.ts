/**
 * The capture flow, apart from the DOM and from Tauri.
 *
 * Constraint 1 states that a capture taking longer than five seconds must
 * revert, and that the user must never be blocked waiting for one. The overlay
 * is where that promise is kept, so the rule lives here rather than in
 * `constraints.rs`.
 */

/** How long the overlay waits for word back before it stops waiting. */
export const CAPTURE_TIMEOUT_MS = 5000;

/** The only thing the overlay ever says. */
export const STATUS_CAPTURED = "Captured";

/**
 * What became of a capture.
 *
 * `unconfirmed` is not a failure. The queue may well hold the event; the
 * overlay simply stopped waiting to hear so.
 */
export type CaptureOutcome = "captured" | "failed" | "unconfirmed";

/**
 * Send a capture, and wait no longer than `timeoutMs` for word back.
 *
 * The rejection is handled at the send rather than left to the race. A send
 * that fails after the overlay has given up therefore cannot surface as an
 * unhandled rejection.
 */
export async function runCapture(
  send: () => Promise<unknown>,
  timeoutMs: number = CAPTURE_TIMEOUT_MS,
): Promise<CaptureOutcome> {
  let timer: ReturnType<typeof setTimeout> | undefined;

  const delivery = send().then(
    (): CaptureOutcome => "captured",
    (): CaptureOutcome => "failed",
  );

  const expiry = new Promise<CaptureOutcome>((resolve) => {
    timer = setTimeout(() => resolve("unconfirmed"), timeoutMs);
  });

  try {
    return await Promise.race([delivery, expiry]);
  } finally {
    clearTimeout(timer);
  }
}

/**
 * Whether the capture took what was in the box.
 *
 * A note always takes it: the text in the box is the note. A marker takes it
 * when the box held a label, and leaves it alone when the box was empty.
 *
 * This used to be the kind of capture, which was the wrong question. Clearing
 * belongs to the capture that consumed the text, whichever gesture that was.
 */
export type TookTheBox = boolean;

/** What the overlay does about an outcome. */
export interface CaptureEffect {
  /** Show the confirmation. */
  flash: boolean;
  /** Put the overlay away. */
  hideWindow: boolean;
  /** Take the captured text off the screen. */
  clearInput: boolean;
}

/**
 * Only a confirmed capture changes what the user sees.
 *
 * A failure and a timeout leave the overlay as it is: open, with the text
 * still in it. Section 9 sets this — the overlay shows no error dialogs and no
 * failure states, and the revert is what keeps the user unblocked. The text on
 * screen is also the only copy anybody can point to, so an unproven capture
 * never clears it.
 *
 * The box is cleared by the capture that took it, and only once that capture
 * is confirmed. Both halves matter. A marker used to clear the box whatever it
 * held, so text typed and never captured was erased by a gesture that had
 * nothing to do with it. A marker that carries the text as its label has
 * everything to do with it, and leaving it behind would invite the same text
 * being sent twice.
 */
export function effectFor(
  outcome: CaptureOutcome,
  tookTheBox: TookTheBox,
): CaptureEffect {
  const confirmed = outcome === "captured";

  return {
    flash: confirmed,
    hideWindow: confirmed,
    clearInput: confirmed && tookTheBox,
  };
}
