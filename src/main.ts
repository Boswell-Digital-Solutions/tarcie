import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  CAPTURE_TIMEOUT_MS,
  STATUS_CAPTURED,
  effectFor,
  runCapture,
} from "./capture";

const win = getCurrentWebviewWindow();

const input = document.querySelector<HTMLInputElement>("#tarcie-input")!;
const markerBtn = document.querySelector<HTMLButtonElement>("#tarcie-marker")!;
const statusEl = document.querySelector<HTMLDivElement>("#tarcie-status")!;

/** How long the confirmation stays up before the overlay goes away. */
const FLASH_MS = 200;

async function capture(send: () => Promise<unknown>) {
  const effect = effectFor(await runCapture(send, CAPTURE_TIMEOUT_MS));

  if (!effect.flash) {
    // Nothing confirmed the capture. The overlay stays exactly as it is, open
    // and holding the text, and says nothing about it.
    return;
  }

  document.body.classList.add("captured");
  statusEl.textContent = STATUS_CAPTURED;

  setTimeout(async () => {
    document.body.classList.remove("captured");
    statusEl.textContent = "";
    if (effect.hideWindow) {
      await win.hide();
    }
    if (effect.clearInput) {
      input.value = "";
    }
  }, FLASH_MS);
}

function captureNote() {
  const content = input.value || "";
  return capture(() => invoke("capture_note", { content }));
}

function captureMarker() {
  return capture(() => invoke("capture_marker", { reason: null }));
}

input.addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    e.preventDefault();
    captureNote().catch(console.error);
  } else if (e.key === "Escape") {
    win.hide().catch(console.error);
  }
});

markerBtn.addEventListener("click", () => {
  captureMarker().catch(console.error);
});

input.focus();
