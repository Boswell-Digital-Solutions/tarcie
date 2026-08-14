import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { wireOverlay } from "./overlay";

const win = getCurrentWebviewWindow();

wireOverlay({
  input: document.querySelector<HTMLInputElement>("#tarcie-input")!,
  marker: document.querySelector<HTMLButtonElement>("#tarcie-marker")!,
  status: document.querySelector<HTMLDivElement>("#tarcie-status")!,
  body: document.body,
  invoke: (command, args) => invoke(command, args),
  hide: () => win.hide(),
});
