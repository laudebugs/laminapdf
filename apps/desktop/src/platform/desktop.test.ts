import { fakePlatform } from "@laminapdf/ui/testing";
import { describe, expect, it } from "vitest";
import { createDesktopPlatform, DESKTOP_FLAGS, isNotYetSupportedError, NotYetSupportedError } from "./desktop.ts";

const PRINT_JOB = { pages: [1], render: () => Promise.reject(new Error("unused")), scale: "fit" } as const;

describe("the desktop platform", () => {
  it("sets the desktop shell's four flags over the webview's", () => {
    const web = fakePlatform({ flags: { canWriteInPlace: false, multiWindow: false } });
    expect(createDesktopPlatform(web).capabilities).toMatchObject(DESKTOP_FLAGS);
  });

  it("keeps the webview's other flags", () => {
    const web = fakePlatform({ flags: { canShare: true } });
    expect(createDesktopPlatform(web).capabilities.canShare).toBe(true);
  });

  it("has nothing to enable for its own flags", () => {
    expect(createDesktopPlatform(fakePlatform()).howToEnable("hasNativePrint")).toBeUndefined();
  });

  it("asks the web platform how to enable the other flags", () => {
    const web = fakePlatform({ flags: { canShare: false }, reasons: { canShare: "web reason" } });
    expect(createDesktopPlatform(web).howToEnable("canShare")).toBe(web.howToEnable("canShare"));
  });

  it("rejects print with NotYetSupportedError naming the feature", async () => {
    const desktop = createDesktopPlatform(fakePlatform());
    await expect(desktop.print(PRINT_JOB)).rejects.toMatchObject({ code: "not-yet-supported", feature: "print" });
  });

  it("rejects saving in place", async () => {
    const desktop = createDesktopPlatform(fakePlatform());
    await expect(desktop.saveInPlace({} as never, new Uint8Array())).rejects.toBeInstanceOf(NotYetSupportedError);
  });

  it("rejects opening a window", async () => {
    const desktop = createDesktopPlatform(fakePlatform());
    await expect(desktop.openWindow?.({} as never)).rejects.toBeInstanceOf(NotYetSupportedError);
  });

  it("throws when asked to watch a file", () => {
    const desktop = createDesktopPlatform(fakePlatform());
    expect(() => desktop.watch?.({} as never, () => undefined)).toThrow(NotYetSupportedError);
  });
});

describe("isNotYetSupportedError", () => {
  it("tells the desktop error from any other", () => {
    expect([new NotYetSupportedError("open"), new Error("other")].map(isNotYetSupportedError)).toEqual([true, false]);
  });
});
