// A document's fingerprint, the key of its reading position: the same for the same file and for
// that file after an incremental update, different after a full rewrite and between files.
// The PDFs are read through Vite as data URLs, so the test needs no Node API.
import { describe, expect, it } from "vitest";
import { documentFingerprint, fingerprintedBytes, HEAD_BYTES } from "../src/worker/fingerprint.js";

const pdfs = import.meta.glob<string>(["../../../fixtures/golden/*.pdf", "../../../fixtures/corpus/*.pdf"], {
  query: "?inline",
  import: "default",
  eager: true,
});

/** Repository file `path`'s bytes. */
async function bytes(path: string): Promise<Uint8Array> {
  const url = pdfs[`../../../${path}`];
  if (url === undefined) throw new Error(`${path} is not read by the test`);
  return new Uint8Array(await (await fetch(url)).arrayBuffer());
}

/** `path`'s fingerprint, from its first {@link HEAD_BYTES} as the worker reads them. */
async function fingerprintOf(path: string): Promise<string> {
  return documentFingerprint((await bytes(path)).subarray(0, HEAD_BYTES));
}

describe("documentFingerprint", () => {
  it("gives the same bytes the same value, a SHA-256 under the fp1 prefix", async () => {
    const first = await fingerprintOf("fixtures/golden/hello.pdf");
    expect(first).toMatch(/^fp1:[0-9a-f]{64}$/);
    expect(await fingerprintOf("fixtures/golden/hello.pdf")).toBe(first);
  });

  it("keeps a file's value through an incremental update that adds an /ID", async () => {
    expect(await fingerprintOf("fixtures/golden/incremental-set-info.pdf")).toBe(
      await fingerprintOf("fixtures/golden/hello.pdf"),
    );
  });

  it("changes after a full rewrite", async () => {
    expect(await fingerprintOf("fixtures/golden/full-rewrite-hello.pdf")).not.toBe(
      await fingerprintOf("fixtures/golden/hello.pdf"),
    );
  });

  it("tells two different files apart", async () => {
    expect(await fingerprintOf("fixtures/corpus/fpdf2.pdf")).not.toBe(
      await fingerprintOf("fixtures/corpus/outline.pdf"),
    );
  });

  it("covers the first revision only, through its %%EOF", async () => {
    const file = await bytes("fixtures/golden/incremental-set-info.pdf");
    const covered = fingerprintedBytes(file);
    expect(new TextDecoder().decode(covered.subarray(-5))).toBe("%%EOF");
    expect(covered.length).toBeLessThan(file.length);
  });

  it("hashes the first 64 KiB of a file larger than that with no marker in them", async () => {
    const large = new Uint8Array(HEAD_BYTES + 1000).fill(0x20);
    const changedLate = large.slice();
    changedLate[HEAD_BYTES + 10] = 0x41;
    const changedEarly = large.slice();
    changedEarly[10] = 0x41;
    expect(fingerprintedBytes(large)).toHaveLength(large.length);
    expect(await documentFingerprint(changedLate)).toBe(await documentFingerprint(large));
    expect(await documentFingerprint(changedEarly)).not.toBe(await documentFingerprint(large));
  });
});
