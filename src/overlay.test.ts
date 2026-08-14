// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { CAPTURE_TIMEOUT_MS, STATUS_CAPTURED } from "./capture";
import { FLASH_MS, wireOverlay } from "./overlay";

interface Harness {
  input: HTMLInputElement;
  marker: HTMLButtonElement;
  status: HTMLElement;
  invoke: ReturnType<typeof vi.fn>;
  hide: ReturnType<typeof vi.fn>;
}

/** An overlay wired to a document the test built and calls it can drive. */
function overlay(invoke = vi.fn().mockResolvedValue(undefined)): Harness {
  document.body.innerHTML = `
    <input id="tarcie-input" />
    <button id="tarcie-marker"></button>
    <div id="tarcie-status"></div>
  `;

  const input = document.querySelector<HTMLInputElement>("#tarcie-input")!;
  const marker = document.querySelector<HTMLButtonElement>("#tarcie-marker")!;
  const status = document.querySelector<HTMLElement>("#tarcie-status")!;
  const hide = vi.fn().mockResolvedValue(undefined);

  wireOverlay({ input, marker, status, body: document.body, invoke, hide });

  return { input, marker, status, invoke, hide };
}

function press(input: HTMLInputElement, key: string): KeyboardEvent {
  const event = new KeyboardEvent("keydown", { key, cancelable: true });
  input.dispatchEvent(event);
  return event;
}

/** Let the pending promises run without moving the clock. */
async function settle() {
  await vi.advanceTimersByTimeAsync(0);
}

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("capturing a note", () => {
  it("sends what was on screen when the key was pressed", async () => {
    const h = overlay();
    h.input.value = "#meeting ship it";

    press(h.input, "Enter");
    await settle();

    expect(h.invoke).toHaveBeenCalledWith("capture_note", {
      content: "#meeting ship it",
    });
  });

  it("does not let the keystroke do anything else", async () => {
    const h = overlay();

    const event = press(h.input, "Enter");
    await settle();

    expect(event.defaultPrevented).toBe(true);
  });

  it("confirms, then puts the overlay away and clears the box", async () => {
    const h = overlay();
    h.input.value = "a note";

    press(h.input, "Enter");
    await settle();

    expect(h.status.textContent).toBe(STATUS_CAPTURED);
    expect(h.input.value, "the text stays up during the flash").toBe("a note");
    expect(h.hide).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(FLASH_MS);

    expect(h.hide).toHaveBeenCalledTimes(1);
    expect(h.input.value).toBe("");
    expect(h.status.textContent).toBe("");
  });
});

describe("a capture nothing confirmed", () => {
  it("leaves the overlay alone when the capture was refused", async () => {
    const h = overlay(vi.fn().mockRejectedValue(new Error("append failed")));
    h.input.value = "a note";

    press(h.input, "Enter");
    await settle();
    await vi.advanceTimersByTimeAsync(FLASH_MS);

    expect(h.hide).not.toHaveBeenCalled();
    expect(h.input.value, "the only copy anyone has is kept").toBe("a note");
    expect(h.status.textContent, "and section 9 rules out saying so").toBe("");
  });

  it("leaves the overlay alone when the capture outlived its budget", async () => {
    const h = overlay(vi.fn().mockReturnValue(new Promise(() => {})));
    h.input.value = "a note";

    press(h.input, "Enter");
    await vi.advanceTimersByTimeAsync(CAPTURE_TIMEOUT_MS);
    await vi.advanceTimersByTimeAsync(FLASH_MS);

    expect(h.hide).not.toHaveBeenCalled();
    expect(h.input.value).toBe("a note");
  });

  it("never takes text the user typed after it gave up", async () => {
    // The reason constraint 1 has teeth. Before the revert, a command that
    // answered late still confirmed, hid the window, and cleared the box —
    // over whatever had been typed in the meantime.
    let settled = (): void => {};
    const pending = new Promise<void>((resolve) => {
      settled = resolve;
    });
    const h = overlay(vi.fn().mockReturnValue(pending));
    h.input.value = "first note";

    press(h.input, "Enter");
    await vi.advanceTimersByTimeAsync(CAPTURE_TIMEOUT_MS);

    h.input.value = "something else entirely";
    settled();
    await vi.advanceTimersByTimeAsync(FLASH_MS);

    expect(h.input.value).toBe("something else entirely");
    expect(h.hide).not.toHaveBeenCalled();
  });
});

describe("the other gestures", () => {
  it("puts the overlay away on Escape without capturing", async () => {
    const h = overlay();
    h.input.value = "not ready yet";

    press(h.input, "Escape");
    await settle();

    expect(h.hide).toHaveBeenCalledTimes(1);
    expect(h.invoke).not.toHaveBeenCalled();
    expect(h.input.value).toBe("not ready yet");
  });

  it("captures a marker with no reason from the button", async () => {
    const h = overlay();

    h.marker.click();
    await settle();

    expect(h.invoke).toHaveBeenCalledWith("capture_marker", { reason: null });
  });

  it("KNOWN DEVIATION: a marker clears a note the user never captured", async () => {
    // Deviation: the marker button is a separate action, but a confirmed
    // capture of any kind clears the box. Text typed and not yet captured is
    // therefore erased by clicking the marker, and the window closes over it.
    // Everywhere else the overlay treats that text as the only copy anyone
    // has. This test records the behaviour as it stands; a fix changes it in
    // the same commit.
    const h = overlay();
    h.input.value = "a note never captured";

    h.marker.click();
    await settle();
    await vi.advanceTimersByTimeAsync(FLASH_MS);

    expect(h.input.value).toBe("");
    expect(h.hide).toHaveBeenCalledTimes(1);
  });

  it("arrives ready to be typed into", () => {
    const h = overlay();

    expect(document.activeElement).toBe(h.input);
  });
});
