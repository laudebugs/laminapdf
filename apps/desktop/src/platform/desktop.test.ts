import { fakePlatform } from "@laminapdf/ui/testing";
import { describe, expect, it } from "vitest";
import { createDesktopPlatform, DESKTOP_FLAGS, NotYetSupportedError } from "./desktop.ts";

describe("the desktop platform", () => {
  it("reports the desktop shell's four capabilities over the webview's", () => {
    const web = fakePlatform({ flags: { canWriteInPlace: false, multiWindow: false, canShare: true } });
    const desktop = createDesktopPlatform(web);
    expect(desktop.capabilities).toMatchObject({ ...DESKTOP_FLAGS, canShare: true });
  });

  it("has nothing to enable for its own flags, and asks the web platform for the rest", () => {
    const web = fakePlatform({ flags: { canShare: false }, reasons: { canShare: "web reason" } });
    const desktop = createDesktopPlatform(web);
    expect(desktop.howToEnable("hasNativePrint")).toBeUndefined();
    expect(desktop.howToEnable("canShare")).toBe(web.howToEnable("canShare"));
  });

  it("rejects what later todos build with NotYetSupportedError, naming the todo", async () => {
    const desktop = createDesktopPlatform(fakePlatform());
    const job = { pages: [1], render: () => Promise.reject(new Error("unused")), scale: "fit" } as const;
    await expect(desktop.print(job)).rejects.toMatchObject({ name: "NotYetSupportedError", todo: "D4c" });
    await expect(desktop.openWindow?.({} as never)).rejects.toBeInstanceOf(NotYetSupportedError);
    expect(() => desktop.watch?.({} as never, () => undefined)).toThrow(NotYetSupportedError);
  });
});
