// Which recent entries need the picker, and when the list explains why.

import type { RecentFile } from "@laminapdf/ui";
import { describe, expect, it } from "vitest";
import { needsPicker, newestReady, wantsPickerNote } from "./recent-picker.ts";

const entry = (id: string, flags: Partial<RecentFile>): RecentFile => ({
  id,
  name: `${id}.pdf`,
  size: 1,
  lastOpened: 0,
  ...flags,
});
const READY = entry("ready", { hasHandle: true, missing: false });
const CHOOSE = entry("choose", { hasHandle: false, missing: false });
const GONE = entry("gone", { hasHandle: false, missing: true });

describe("an entry that needs the picker", () => {
  it("is a file that is there without a handle", () => {
    expect([READY, CHOOSE, GONE].map(needsPicker)).toEqual([false, true, false]);
  });

  it("is not the one the palette opens", () => {
    expect(newestReady([GONE, CHOOSE, READY])?.id).toBe("ready");
  });
});

describe("the note saying why", () => {
  it("shows the first time an entry needs the picker", () => {
    expect(wantsPickerNote([READY, CHOOSE], false)).toBe(true);
  });

  it("stays away once seen", () => {
    expect(wantsPickerNote([CHOOSE], true)).toBe(false);
  });

  it("waits until the stored choice is read", () => {
    expect(wantsPickerNote([CHOOSE], undefined)).toBe(false);
  });

  it("stays away when every file opens without it", () => {
    expect(wantsPickerNote([READY, GONE], false)).toBe(false);
  });
});
