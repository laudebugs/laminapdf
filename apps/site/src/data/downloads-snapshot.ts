// Reads the download page's snapshot at build time (src/data/downloads.ts says where it comes from).
// The committed snapshot is imported, so the bundled page carries it; LAMINA_PDF_DOWNLOADS names a
// file to read instead, relative to where the build runs.
import { readFileSync } from "node:fs";
import process from "node:process";
import { parseSnapshot, type Snapshot } from "./downloads.js";
import committed from "./downloads.json";

/** The snapshot the page is built from: LAMINA_PDF_DOWNLOADS's file, or the committed one. */
export function loadSnapshot(env: Readonly<Record<string, string | undefined>> = process.env): Snapshot {
  const file = env.LAMINA_PDF_DOWNLOADS;
  return parseSnapshot(file === undefined ? JSON.stringify(committed) : readFileSync(file, "utf8"));
}
