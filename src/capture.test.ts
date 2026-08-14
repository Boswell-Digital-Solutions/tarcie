import { describe, expect, it, vi } from "vitest";
import {
  CAPTURE_TIMEOUT_MS,
  effectFor,
  runCapture,
  type CaptureOutcome,
} from "./capture";

/** Run `body` with the clock under the test's control. */
async function withFakeTimers(body: () => Promise<void>) {
  vi.useFakeTimers();
  try {
    await body();
  } finally {
    vi.useRealTimers();
  }
}

describe("runCapture", () => {
  it("reports a capture the queue confirmed", async () => {
    await expect(runCapture(() => Promise.resolve())).resolves.toBe("captured");
  });

  it("reports a refusal instead of claiming success", async () => {
    const send = () => Promise.reject(new Error("append failed"));

    await expect(runCapture(send)).resolves.toBe("failed");
  });

  it("stops waiting once the budget is spent", async () => {
    // Constraint 1: the user is never left waiting on a capture.
    await withFakeTimers(async () => {
      const outcome = runCapture(() => new Promise(() => {}), CAPTURE_TIMEOUT_MS);

      await vi.advanceTimersByTimeAsync(CAPTURE_TIMEOUT_MS);

      await expect(outcome).resolves.toBe("unconfirmed");
    });
  });

  it("waits the whole budget before it gives up", async () => {
    // A slow capture that still lands inside the budget is a capture.
    await withFakeTimers(async () => {
      let settle = (): void => {};
      const pending = new Promise<void>((resolve) => {
        settle = resolve;
      });
      const outcome = runCapture(() => pending, CAPTURE_TIMEOUT_MS);

      await vi.advanceTimersByTimeAsync(CAPTURE_TIMEOUT_MS - 1);
      settle();

      await expect(outcome).resolves.toBe("captured");
    });
  });

  it("leaves no timer behind when the capture lands first", async () => {
    await withFakeTimers(async () => {
      await runCapture(() => Promise.resolve(), CAPTURE_TIMEOUT_MS);

      expect(vi.getTimerCount()).toBe(0);
    });
  });

  it("survives a send that fails after the overlay gave up", async () => {
    // The rejection arrives with nothing waiting on it. Because it is handled
    // at the send, it cannot become an unhandled rejection.
    await withFakeTimers(async () => {
      let fail = (_reason: Error): void => {};
      const pending = new Promise<void>((_resolve, reject) => {
        fail = reject;
      });
      const outcome = runCapture(() => pending, CAPTURE_TIMEOUT_MS);

      await vi.advanceTimersByTimeAsync(CAPTURE_TIMEOUT_MS);
      await expect(outcome).resolves.toBe("unconfirmed");

      fail(new Error("too late"));
      await vi.advanceTimersByTimeAsync(0);
    });
  });
});

describe("effectFor", () => {
  it("confirms, hides, and clears when the capture was confirmed", () => {
    expect(effectFor("captured")).toEqual({
      flash: true,
      hideWindow: true,
      clearInput: true,
    });
  });

  it("leaves the overlay as it is when nothing confirmed the capture", () => {
    // The overlay holds the only copy anyone can point to, so an unproven
    // capture never clears it — and section 9 rules out saying so on screen.
    for (const outcome of ["failed", "unconfirmed"] as const) {
      const effect = effectFor(outcome);

      expect(effect.flash, outcome).toBe(false);
      expect(effect.hideWindow, outcome).toBe(false);
      expect(effect.clearInput, outcome).toBe(false);
    }
  });

  it("never clears the text without also having confirmed the capture", () => {
    const outcomes: CaptureOutcome[] = ["captured", "failed", "unconfirmed"];

    for (const outcome of outcomes) {
      const effect = effectFor(outcome);

      expect(effect.clearInput && !effect.flash, outcome).toBe(false);
    }
  });
});
