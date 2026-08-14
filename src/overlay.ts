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

  async function capture(send: () => Promise<unknown>) {
    const effect = effectFor(await runCapture(send, CAPTURE_TIMEOUT_MS));

    if (!effect.flash) {
      // Nothing confirmed the capture. The overlay stays exactly as it is,
      // open and holding the text, and says nothing about it.
      return;
    }

    body.classList.add("captured");
    status.textContent = STATUS_CAPTURED;

    setTimeout(async () => {
      body.classList.remove("captured");
      status.textContent = "";
      if (effect.hideWindow) {
        await hide();
      }
      if (effect.clearInput) {
        input.value = "";
      }
    }, FLASH_MS);
  }

  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      // Read at the keystroke. What the capture carries is what was on screen
      // when the user asked for it.
      const content = input.value || "";
      capture(() => invoke("capture_note", { content })).catch(console.error);
    } else if (e.key === "Escape") {
      hide().catch(console.error);
    }
  });

  marker.addEventListener("click", () => {
    capture(() => invoke("capture_marker", { reason: null })).catch(
      console.error,
    );
  });

  // The overlay exists to be typed into. It arrives ready.
  input.focus();
}
