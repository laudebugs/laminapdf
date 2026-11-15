import { type Platform, ToastQueue } from "@laminapdf/ui";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { copier } from "../src/copy.ts";

// Every Copy button in the reader says it worked with one toast, "Copied"; a copy the clipboard
// refuses shows no toast and is reported instead.

/** A platform whose clipboard takes the text, or refuses it. */
function platform(copyText: (text: string) => Promise<void>): Platform {
  return { copyText } as unknown as Platform;
}

// The browser's reportError, which Node lacks.
const reported = vi.fn();
beforeEach(() => vi.stubGlobal("reportError", reported));
afterEach(() => {
  vi.unstubAllGlobals();
  reported.mockClear();
});

describe("copier", () => {
  it("copies the text, then says Copied", async () => {
    const copied: string[] = [];
    const toasts = new ToastQueue();
    copier(
      platform(async (text) => {
        copied.push(text);
      }),
      toasts,
    )("E-123: broken xref");
    await vi.waitFor(() => expect(toasts.entries()).toHaveLength(1));
    expect(copied).toEqual(["E-123: broken xref"]);
    expect(toasts.entries()[0]?.options.message).toBe("Copied");
    expect(reported).not.toHaveBeenCalled();
  });

  it("reports a copy the clipboard refused, and says nothing", async () => {
    const toasts = new ToastQueue();
    const refusal = new Error("denied");
    copier(
      platform(() => Promise.reject(refusal)),
      toasts,
    )("text");
    await vi.waitFor(() => expect(reported).toHaveBeenCalledWith(refusal));
    expect(toasts.entries()).toHaveLength(0);
  });
});
