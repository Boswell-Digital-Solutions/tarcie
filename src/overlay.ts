/**
 * The overlay wiring: which gesture starts which capture, and what the window
 * does about the outcome.
 *
 * The decisions live in `capture.ts`. This module connects them to a keyboard,
 * a button, and a window.
 */

import {
  CAPTURE_TIMEOUT_MS,
  STATUS_CAPTURED,
  effectFor,
  runCapture,
  type TookTheBox,
} from "./capture";

/** How long the confirmation stays up before the overlay goes away. */
export const FLASH_MS = 200;

/**
 * Everything the overlay touches outside itself.
 *
 * `main.ts` supplies the real elements and the real Tauri calls. A test
 * supplies a document it built and calls it can drive, so the wiring is
 * provable without a running application.
 */
export interface OverlayPorts {
  input: HTMLInputElement;
  marker: HTMLButtonElement;
  status: HTMLElement;
  body: HTMLElement;
  invoke: (command: string, args: Record<string, unknown>) => Promise<unknown>;
  hide: () => Promise<void>;
}

export function wireOverlay(ports: OverlayPorts): void {
  const { input, marker, status, body, invoke, hide } = ports;

  /**
   * One capture at a time.
   *
   * The box is not cleared until the flash ends, so a second Enter inside that
   * window sent the same text again under a fresh id. Deduplication downstream
   * is on `id`, so nothing would ever catch the pair.
   *
   * A gesture the guard turns away costs nothing. The text stays on screen,
   * which is where every unconfirmed capture leaves it anyway, and the next
   * gesture is taken as soon as this one is done.
   */
  let capturing = false;

  async function capture(tookTheBox: TookTheBox, send: () => Promise<unknown>) {
    if (capturing) {
      return;
    }
    capturing = true;

    try {
      const effect = effectFor(
        await runCapture(send, CAPTURE_TIMEOUT_MS),
        tookTheBox,
      );

      if (!effect.flash) {
        // Nothing confirmed the capture. The overlay stays exactly as it is,
        // open and holding the text, and says nothing about it.
        return;
      }

      body.classList.add("captured");
      status.textContent = STATUS_CAPTURED;

      await new Promise<void>((resolve) => {
        setTimeout(async () => {
          body.classList.remove("captured");
          status.textContent = "";
          if (effect.hideWindow) {
            await hide();
          }
          if (effect.clearInput) {
            input.value = "";
          }
          resolve();
        }, FLASH_MS);
      });
    } finally {
      capturing = false;
    }
  }

  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      // Read at the keystroke. What the capture carries is what was on screen
      // when the user asked for it.
      const content = input.value || "";

      // Nothing typed is nothing to capture. The hotkey opens the overlay
      // ready for typing, so a reflexive Enter would otherwise queue an event
      // with no content, deliver it, and archive it for good.
      //
      // A tag on its own is not nothing. It is the only way to mark a moment
      // with a label, because a marker carries no tag, so it goes through.
      if (content.trim() === "") {
        return;
      }

      // A note is the text in the box, so it always takes it.
      capture(true, () => invoke("capture_note", { content })).catch(
        console.error,
      );
    } else if (e.key === "Escape") {
      hide().catch(console.error);
    }
  });

  marker.addEventListener("click", () => {
    // Whatever is in the box becomes the marker's label. The gesture is the
    // observation, so a marker needs no text — an empty box still marks the
    // moment, and that is the only way left to say "here, and nothing more".
    const reason = input.value.trim() === "" ? null : input.value;

    capture(reason !== null, () =>
      invoke("capture_marker", { reason }),
    ).catch(console.error);
  });

  // The overlay exists to be typed into. It arrives ready.
  input.focus();
}
