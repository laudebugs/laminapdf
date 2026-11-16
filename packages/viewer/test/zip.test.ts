import { isLaminaPdfError } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import { crc32 } from "../src/worker/crc32.js";
import { zip } from "../src/worker/zip.js";
import { entries } from "./zip-entries.js";

describe("zip", () => {
  it("stores each file whole, in order, under its UTF-8 name with its CRC-32", () => {
    const files = [
      { name: "Q3 report – part 1.pdf", bytes: new TextEncoder().encode("%PDF-1.7 one") },
      { name: "Rapport été.pdf", bytes: new Uint8Array([0, 1, 2, 255]) },
      { name: "empty.pdf", bytes: new Uint8Array() },
    ];
    const read = entries(zip(files, new Date(2026, 10, 15, 9, 30, 42)));
    expect(read.map((e) => e.name)).toEqual(files.map((f) => f.name));
    read.forEach((entry, i) => {
      const file = files[i] as (typeof files)[number];
      expect(entry.data).toEqual(file.bytes);
      expect(entry.crc).toBe(crc32(file.bytes));
    });
  });

  it("computes the CRC-32 of the standard check value", () => {
    expect(crc32(new TextEncoder().encode("123456789"))).toBe(0xcbf43926);
  });

  it("writes the MS-DOS date and time of the modification", () => {
    const bytes = zip([{ name: "a.pdf", bytes: new Uint8Array([1]) }], new Date(2026, 10, 15, 9, 30, 42));
    const view = new DataView(bytes.buffer);
    expect(view.getUint16(10, true)).toBe((9 << 11) | (30 << 5) | 21);
    expect(view.getUint16(12, true)).toBe(((2026 - 1980) << 9) | (11 << 5) | 15);
  });

  it("refuses more files than a zip without ZIP64 holds", () => {
    const many = Array.from({ length: 0x10000 }, (_, i) => ({ name: `${i}.pdf`, bytes: new Uint8Array() }));
    let thrown: unknown;
    try {
      zip(many, new Date());
    } catch (e) {
      thrown = e;
    }
    expect(isLaminaPdfError(thrown, "limit-exceeded")).toBe(true);
  });
});
