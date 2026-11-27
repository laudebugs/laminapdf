// The files the touched-page reload is measured on and its baselines (reload.json), shared with
// the desktop's bench: the sizes live in one file.

import { readFileSync } from "node:fs";

export interface ReloadFile {
  readonly megabytes: number;
  readonly fixture: string;
  readonly pages: number;
  readonly side: number;
}

interface ReloadShapes {
  readonly files: readonly ReloadFile[];
  readonly baselinesMs: Readonly<Record<"web" | "desktop", Readonly<Record<string, number | null>>>>;
}

const shapes = JSON.parse(readFileSync(new URL("reload.json", import.meta.url), "utf8")) as ReloadShapes;

export const RELOAD_FILES = shapes.files;
export const RELOAD_SIZES = shapes.files.map((file) => file.megabytes);

/** The web's baseline for the `megabytes` file, in ms: the CI runner's median, or null while none is recorded. */
export const webBaselineMs = (megabytes: number): number | null => shapes.baselinesMs.web[String(megabytes)] ?? null;

/** The most a gated figure may be over its baseline: 20 %, and at least 10 ms for the small ones. */
export const limitMs = (baseline: number): number => Math.max(baseline * 1.2, baseline + 10);
