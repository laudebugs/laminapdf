// The export dialog's streaming zip: files added one at a time as Blob parts, read back from
// the central directory, and checked by the system's own unzip.
// biome-ignore-all lint/correctness/noNodejsModules: this Node-only test writes the zip to a file for the system's unzip.
import { execFileSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { isLaminaPdfError } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import { crc32 } from "../src/worker/crc32.js";
import { ZipWriter } from "../src/zip/writer.js";
import { entries } from "./zip-entries.js";

const FILES = [
  { name: "page-1.png", bytes: new Uint8Array([0x89, 0x50, 0x4e, 0x47, 1, 2, 3]) },
  { name: "Rapport été/page 2.txt", bytes: new TextEncoder().encode("hello, zip") },
  { name: "empty.json", bytes: new Uint8Array() },
];

async function written(files = FILES): Promise<Uint8Array> {
  const writer = new ZipWriter(new Date(2026, 10, 16, 12, 0, 0));
  for (const file of files) writer.add(file.name, file.bytes);
  return new Uint8Array(await writer.finish().arrayBuffer());
}

/** What `f` threw. */
function thrown(f: () => unknown): unknown {
  try {
    f();
  } catch (e) {
    return e;
  }
  return undefined;
}

/** Whether `unzip` is on the PATH. */
function hasUnzip(): boolean {
  try {
    execFileSync("unzip", ["-v"], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

describe("ZipWriter", () => {
  it("stores each file whole, in order, under its UTF-8 name with its CRC-32", async () => {
    const read = entries(await written());
    expect(read.map((e) => e.name)).toEqual(FILES.map((f) => f.name));
    read.forEach((entry, i) => {
      const file = FILES[i] as (typeof FILES)[number];
      expect(entry.data).toEqual(file.bytes);
      expect(entry.crc).toBe(crc32(file.bytes));
    });
  });

  it("counts the bytes of the files added and their headers in size", () => {
    const writer = new ZipWriter(new Date());
    writer.add("a.txt", new Uint8Array(10));
    expect(writer.size).toBe(30 + "a.txt".length + 10);
  });

  it("writes a zip whose size is size plus its directory", async () => {
    const writer = new ZipWriter(new Date());
    writer.add("a.txt", new Uint8Array(10));
    const before = writer.size;
    expect(writer.finish().size).toBe(before + 46 + "a.txt".length + 22);
  });

  it.skipIf(!hasUnzip())("writes a zip the system's unzip -t passes (skipped without unzip on the PATH)", async () => {
    const folder = mkdtempSync(join(tmpdir(), "lamina-zip-"));
    try {
      const path = join(folder, "export.zip");
      writeFileSync(path, await written());
      const report = execFileSync("unzip", ["-t", path], { encoding: "utf8" });
      expect(report).toContain("No errors detected");
    } finally {
      rmSync(folder, { recursive: true, force: true });
    }
  });

  it("refuses a name it already holds", () => {
    const writer = new ZipWriter(new Date());
    writer.add("page-1.png", new Uint8Array([1]));
    expect(
      isLaminaPdfError(
        thrown(() => writer.add("page-1.png", new Uint8Array([2]))),
        "invalid-argument",
      ),
    ).toBe(true);
  });

  it("refuses a file past the 65,535 a zip without ZIP64 holds", () => {
    const writer = new ZipWriter(new Date());
    for (let i = 0; i < 0xffff; i++) writer.add(`${i}`, new Uint8Array());
    expect(
      isLaminaPdfError(
        thrown(() => writer.add("one more", new Uint8Array())),
        "limit-exceeded",
      ),
    ).toBe(true);
  });

  it("refuses a file that would take the zip past 4 GiB", () => {
    const writer = new ZipWriter(new Date());
    // Only the length is read before the refusal, so a view claiming 4 GiB costs nothing.
    const huge = { length: 0xffffffff } as Uint8Array;
    expect(
      isLaminaPdfError(
        thrown(() => writer.add("huge.bin", huge)),
        "limit-exceeded",
      ),
    ).toBe(true);
  });

  it("takes no files once finished", () => {
    const writer = new ZipWriter(new Date());
    writer.finish();
    expect(
      isLaminaPdfError(
        thrown(() => writer.add("late.txt", new Uint8Array())),
        "invalid-argument",
      ),
    ).toBe(true);
  });
});
