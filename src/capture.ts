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
 */
export function effectFor(outcome: CaptureOutcome): CaptureEffect {
  const confirmed = outcome === "captured";

  return { flash: confirmed, hideWindow: confirmed, clearInput: confirmed };
}
