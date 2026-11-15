// Reads the download page's snapshot at build time (src/data/downloads.ts says where it comes from).
import { readFileSync } from "node:fs";
import process from "node:process";
import { parseSnapshot, type Snapshot } from "./downloads.js";

/** The snapshot the page is built from: LAMINA_PDF_DOWNLOADS's file, or the committed one. */
export function loadSnapshot(env: Readonly<Record<string, string | undefined>> = process.env): Snapshot {
  const file = env.LAMINA_PDF_DOWNLOADS ?? new URL("./downloads.json", import.meta.url);
  return parseSnapshot(readFileSync(file, "utf8"));
}
