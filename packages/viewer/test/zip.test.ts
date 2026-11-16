import { describe, expect, it } from "vitest";
import { crc32 } from "../src/worker/crc32.js";
import { zip } from "../src/worker/zip.js";

/** The entries a zip's central directory lists, read back as APPNOTE §4.3.12 lays them out. */
function entries(bytes: Uint8Array): { name: string; crc: number; size: number; data: Uint8Array }[] {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const end = bytes.length - 22;
  expect(view.getUint32(end, true)).toBe(0x06054b50);
  const count = view.getUint16(end + 10, true);
  let at = view.getUint32(end + 16, true);
  const out = [];
  for (let i = 0; i < count; i++) {
    expect(view.getUint32(at, true)).toBe(0x02014b50);
    expect(view.getUint16(at + 8, true) & 0x0800).toBe(0x0800);
    const crc = view.getUint32(at + 16, true);
    const size = view.getUint32(at + 24, true);
    const nameLength = view.getUint16(at + 28, true);
    const name = new TextDecoder().decode(bytes.subarray(at + 46, at + 46 + nameLength));
    const local = view.getUint32(at + 42, true);
    expect(view.getUint32(local, true)).toBe(0x04034b50);
    const start = local + 30 + view.getUint16(local + 26, true);
    out.push({ name, crc, size, data: bytes.subarray(start, start + size) });
    at += 46 + nameLength;
  }
  return out;
}

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
    expect(() => zip(many, new Date())).toThrow(RangeError);
  });
});
