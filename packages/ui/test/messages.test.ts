import { describe, expect, it } from "vitest";
import { LATER_ENGLISH, laterMessage } from "../src/later-messages.ts";
import { type Catalogue, ENGLISH, message } from "../src/messages.ts";

describe("message", () => {
  it("returns the English text by default", () => {
    expect(message("platform.picker.pdf")).toBe("PDF documents");
  });

  it("fills each placeholder from the values, and leaves one without a value as written", () => {
    const catalogue = { ...ENGLISH, "platform.picker.pdf": "Page {page} of {total}, {missing}" } as Catalogue;
    expect(message("platform.picker.pdf", { page: 3, total: "12" }, catalogue)).toBe("Page 3 of 12, {missing}");
  });

  it("has no empty message", () => {
    expect(Object.entries(ENGLISH).filter(([, text]) => text.trim() === "")).toEqual([]);
  });
});

describe("laterMessage", () => {
  it("finds a message in either catalogue", () => {
    expect(laterMessage("properties.edit.apply")).toBe("Apply");
    expect(laterMessage("platform.picker.pdf")).toBe("PDF documents");
  });

  it("has no empty message, and no key the reader's catalogue also has", () => {
    expect(Object.entries(LATER_ENGLISH).filter(([, text]) => text.trim() === "")).toEqual([]);
    expect(Object.keys(LATER_ENGLISH).filter((id) => id in ENGLISH)).toEqual([]);
  });
});
