// The size budgets (todo U6b): the preloaded file and all Latin files together. Latin Extended
// files load only for text that needs them, so they are outside the budget.

import { describe, expect, it } from "vitest";
import { FONT_PRELOADS } from "../src/index.ts";
import { lock } from "./helpers.ts";

const PRELOAD_BUDGET = 30 * 1024;
const LATIN_BUDGET = 200 * 1024;

describe("size budgets", () => {
  it("keeps the preloaded file under 30 KB", () => {
    const { outputs } = lock();
    for (const preload of FONT_PRELOADS) {
      const file = `files/${preload.href.split("/").pop()}`;
      expect(outputs[file]?.bytes ?? Number.POSITIVE_INFINITY).toBeLessThanOrEqual(PRELOAD_BUDGET);
    }
  });

  it("keeps every Latin file together under 200 KB", () => {
    const latin = Object.entries(lock().outputs).filter(([file]) => /-latin-(?!ext)/.test(file));
    expect(latin.length).toBe(5);
    expect(latin.reduce((sum, [, output]) => sum + output.bytes, 0)).toBeLessThanOrEqual(LATIN_BUDGET);
  });

  it("preloads Lamina Sans' Latin file and nothing else", () => {
    expect(FONT_PRELOADS.map((p) => [p.href.split("/").pop(), p.type])).toEqual([
      ["lamina-sans-latin-wght-normal.woff2", "font/woff2"],
    ]);
  });
});
