// SPEC case 6 (TS side): no host APIs in src/, and the built ESM loads with a plain import().
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { describe, expect, test } from "vitest";
import { PKG } from "./helpers.js";
import { helloPdf } from "./recipes.js";

const SRC = join(PKG, "src");
const FORBIDDEN: Array<[string, RegExp]> = [
  ["node:", /node:/],
  ["Buffer", /\bBuffer\b/],
  ["require(", /\brequire\s*\(/],
  ["process.", /\bprocess\./],
  ["fs", /\bfs\b/],
];

describe("case 6: portability", () => {
  const files = readdirSync(SRC).filter((f) => f.endsWith(".ts"));

  test("src/ has library files", () => expect(files.length).toBeGreaterThan(5));

  test.each(FORBIDDEN)("src/ never mentions %s", (_, re) => {
    for (const f of files) {
      const lines = readFileSync(join(SRC, f), "utf8").split("\n");
      lines.forEach((l, i) => {
        expect(re.test(l), `${f}:${i + 1}: ${l.trim()}`).toBe(false);
      });
    }
  });

  test("src/ imports only relative modules and fflate", () => {
    for (const f of files) {
      for (const m of readFileSync(join(SRC, f), "utf8").matchAll(/\bfrom\s+"([^"]+)"/g)) {
        expect(m[1]!.startsWith("./") || m[1] === "fflate", `${f}: ${m[1]}`).toBe(true);
      }
    }
  });

  const dist = join(PKG, "dist", "index.js");
  test("built dist/ ESM loads with import() and works", async () => {
    expect(existsSync(dist), "dist/ missing — run `pnpm build` (the `test` script does)").toBe(true);
    const mod = (await import(pathToFileURL(dist).href)) as typeof import("../src/index.js");
    const doc = mod.open(helloPdf());
    expect(doc.pageCount()).toBe(1);
    const out = new mod.PdfWriter();
    out.add(mod.dict([["Type", mod.name("Catalog")]]));
    expect(mod.open(out.toBytes(mod.dict([["Root", mod.ref(1)]]), { mode: "compact" })).wasRepaired()).toBe(false);
  });
});
