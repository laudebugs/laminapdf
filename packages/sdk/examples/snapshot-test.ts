// Checks that a PDF your code makes is byte for byte the reference PDF you approved.
//
// Build the document with `TEST_OPTIONS`, then let your test runner check the answer:
//
//   expect(compareWithReference(invoice(order, TEST_OPTIONS), "references/invoice.pdf")).toBe("same");
//
// The first run fails with `created` and writes the reference: open it, and commit it once it
// looks right. When the bytes change later, the new PDF is written beside the reference for
// review; pass `{ update: true }` to accept it.
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { basename, dirname, extname, join } from "node:path";
import type { BuilderOptions } from "@laminapdf/sdk";

/**
 * What a test passes in place of the clock and a random file ID. With these fixed, the same code
 * and data give the same bytes on every run, machine and runtime.
 */
export const TEST_OPTIONS: BuilderOptions = {
  // The file ID: any 32 hex digits, as long as they stay the same.
  id: "00112233445566778899aabbccddeeff",
  // The Info dictionary's /Producer: your app's name, without a version that changes.
  producer: "my-app tests",
  // The Info dictionary's /CreationDate, in PDF date form.
  creationDate: "D:20260101000000Z",
};

/** `same`: the bytes match. `created`: there was no reference, so one was written. `changed`: they differ. */
export type Outcome = "same" | "created" | "changed";

export interface CompareOptions {
  /** Replace the reference with the new bytes instead of writing them beside it. */
  readonly update?: boolean;
}

/**
 * Compares `bytes` with the reference PDF at `reference`. A changed PDF is written to
 * `<name>.new.pdf` next to the reference, or replaces it when `update` is set.
 */
export function compareWithReference(bytes: Uint8Array, reference: string, options: CompareOptions = {}): Outcome {
  const pending = join(dirname(reference), `${basename(reference, extname(reference))}.new.pdf`);
  if (!existsSync(reference)) {
    mkdirSync(dirname(reference), { recursive: true });
    writeFileSync(reference, bytes);
    return "created";
  }
  if (sameBytes(readFileSync(reference), bytes)) {
    rmSync(pending, { force: true });
    return "same";
  }
  if (options.update) {
    writeFileSync(reference, bytes);
    rmSync(pending, { force: true });
  } else {
    writeFileSync(pending, bytes);
  }
  return "changed";
}

function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
  return a.length === b.length && a.every((byte, i) => byte === b[i]);
}
