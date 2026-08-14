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

  it("sends nothing when there is nothing in the box", async () => {
    // A hotkey opens the overlay ready for typing, so a reflexive Enter costs
    // an event with no content — queued, delivered, and archived for good.
    const h = overlay();
    h.input.value = "   ";

    press(h.input, "Enter");
    await settle();

    expect(h.invoke).not.toHaveBeenCalled();
  });

  it("leaves what counts as a note to the command", async () => {
    // The overlay stops an obviously empty box and stops there. `capture_note`
    // decides the rest, including a tag with nothing behind it, so the tag
    // pattern lives in one place rather than in two that can drift apart.
    const h = overlay();
    h.input.value = "#bug";

    press(h.input, "Enter");
    await settle();

    expect(h.invoke).toHaveBeenCalledWith("capture_note", { content: "#bug" });
  });

  it("keeps the text when the command refuses the note", async () => {
    // A refusal is silent, like every other. The overlay stays open holding
    // what was typed, which is the only copy anyone can point to.
    const h = overlay(vi.fn().mockRejectedValue("a note needs text of its own"));
    h.input.value = "#bug";

    press(h.input, "Enter");
    await vi.advanceTimersByTimeAsync(FLASH_MS);

    expect(h.input.value).toBe("#bug");
    expect(h.hide).not.toHaveBeenCalled();
  });

  it("sends one capture for one gesture, however often Enter is pressed", async () => {
    // The box is not cleared until the flash ends, so a second Enter inside
    // that window sent the same text again under a fresh id. Downstream
    // deduplication is on id, so nothing would ever catch it.
    const h = overlay();
    h.input.value = "a note";

    press(h.input, "Enter");
    press(h.input, "Enter");
    await settle();

    expect(h.invoke).toHaveBeenCalledTimes(1);
  });

  it("takes the next note once the one before it is done", async () => {
    // The guard holds for one capture, never for the next one.
    const h = overlay();
    h.input.value = "first";

    press(h.input, "Enter");
    await vi.advanceTimersByTimeAsync(FLASH_MS);

    h.input.value = "second";
    press(h.input, "Enter");
    await settle();

    expect(h.invoke).toHaveBeenCalledTimes(2);
    expect(h.invoke).toHaveBeenLastCalledWith("capture_note", {
      content: "second",
    });
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

  it("takes what is in the box as the marker's label", async () => {
    // A marker used to ignore the box, which left no way to say what a moment
    // was about once a tag-only note was refused. It now carries the text as
    // its label, so the text is captured rather than passed over.
    const h = overlay();
    h.input.value = "#bug";

    h.marker.click();
    await settle();

    expect(h.invoke).toHaveBeenCalledWith("capture_marker", {
      reason: "#bug",
    });
  });

  it("clears the box once the marker that took it is confirmed", async () => {
    // The text was captured, so leaving it on screen would invite the same
    // label being sent twice.
    const h = overlay();
    h.input.value = "#bug";

    h.marker.click();
    await settle();
    await vi.advanceTimersByTimeAsync(FLASH_MS);

    expect(h.input.value).toBe("");
    expect(h.hide).toHaveBeenCalledTimes(1);
  });

  it("keeps the label when nothing confirmed the marker", async () => {
    // The ground the old rule was really defending: text is never taken off
    // the screen by a capture that was not confirmed to have carried it.
    const h = overlay(vi.fn().mockRejectedValue("no"));
    h.input.value = "#bug";

    h.marker.click();
    await settle();
    await vi.advanceTimersByTimeAsync(FLASH_MS);

    expect(h.input.value).toBe("#bug");
    expect(h.hide).not.toHaveBeenCalled();
  });

  it("still clears the box when the note itself was captured", async () => {
    // The fix is about whose gesture owns the box, not about keeping text
    // after a capture that took it.
    const h = overlay();
    h.input.value = "a note";

    press(h.input, "Enter");
    await settle();
    await vi.advanceTimersByTimeAsync(FLASH_MS);

    expect(h.input.value).toBe("");
  });

  it("arrives ready to be typed into", () => {
    const h = overlay();

    expect(document.activeElement).toBe(h.input);
  });
});
