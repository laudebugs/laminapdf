// What the home page shows from other files: the code panel's tabs are the tested examples and
// the CLI README's commands, and the package table is the docs site's install table, every row.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import progress from "../src/data/progress.json";
import { roundName } from "../src/data/rounds.js";
import { cliCommands, installLines, packages, rustFirstPdf, typescriptFirstPdf, VERSION } from "../src/data/sources.js";

const read = (path: string) => readFileSync(new URL(`../../../${path}`, import.meta.url), "utf8");

describe("the code panel", () => {
  it("shows the TypeScript example without its header comment", () => {
    const example = read("packages/sdk/examples/hello.ts");
    const shown = typescriptFirstPdf();
    expect(example.trimEnd().endsWith(shown)).toBe(true);
    expect(shown.startsWith('import { Builder, type BuilderOptions, Font } from "@laminapdf/sdk";')).toBe(true);
  });

  it("shows the Rust example's import and its build function", () => {
    const example = read("crates/laminapdf/examples/hello.rs");
    const shown = rustFirstPdf();
    const [use = "", , ...build] = shown.split("\n");
    expect(example).toContain(use);
    expect(example.trimEnd().endsWith(build.join("\n"))).toBe(true);
    expect(shown).toContain("pub(crate) fn build(");
    expect(shown).not.toContain("fn main(");
  });

  it("shows each `$` command of the CLI README's examples, without `echo $?`", () => {
    const readme = read("crates/laminapdf-cli/README.md");
    const commands = cliCommands().split("\n");
    expect(commands.length).toBeGreaterThan(3);
    for (const command of commands) expect(readme).toContain(command);
    expect(commands.some((command) => command.includes("echo $?"))).toBe(false);
  });
});

describe("the packages", () => {
  const page = read("apps/docs/src/content/docs/install.mdx");

  it("lists every row of the docs site's install table, in its order", () => {
    const rows = page.split("\n").filter((line) => /^\| `[^`]+` \|/.test(line));
    expect(packages().map((item) => item.name)).toEqual(rows.map((row) => row.match(/`([^`]+)`/)?.[1]));
    expect(packages().map((item) => item.name)).toEqual(expect.arrayContaining(["@laminapdf/sdk", "laminapdf"]));
  });

  it("shows the install page's two install lines and the published version", () => {
    expect(installLines()).toEqual([
      "npm install @laminapdf/sdk@alpha",
      expect.stringMatching(/^cargo add laminapdf@/),
    ]);
    expect(VERSION).toBe(JSON.parse(read("packages/wasm/package.json")).version);
  });
});

describe("the rounds' plain names", () => {
  it("name every section of the board in the snapshot", () => {
    for (const round of progress.rounds) expect(roundName(round.letter)).not.toBe("");
    expect(() => roundName("Z")).toThrow("no plain name");
  });
});
