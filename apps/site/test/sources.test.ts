// What the home page shows from other files: the code panel's tabs are the tested examples and
// the CLI README's commands, and the package table is the docs site's install table, every row.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import progress from "../src/data/progress.json";
import { roundName } from "../src/data/rounds.js";
import {
  alphaNote,
  cliCommands,
  installLines,
  packages,
  rustFirstPdf,
  typescriptFirstPdf,
  VERSION,
} from "../src/data/sources.js";

const read = (path: string) => readFileSync(new URL(`../../../${path}`, import.meta.url), "utf8");

describe("the code panel", () => {
  it("shows the TypeScript example without its header comment", () => {
    const example = read("packages/sdk/examples/hello.ts");
    const shown = typescriptFirstPdf();
    expect(example.trimEnd().endsWith(shown)).toBe(true);
    expect(shown.startsWith('import { Builder, type BuilderOptions, Font } from "@laminapdf/sdk";')).toBe(true);
  });

  it("shows the Rust example's build function with every import it needs, and only those", () => {
    const example = read("crates/laminapdf/examples/hello.rs");
    const shown = rustFirstPdf();
    const uses = shown.split("\n").filter((line) => line.startsWith("use "));
    const build = shown.slice(shown.indexOf("\n\n///") + 2);
    for (const use of uses) expect(example).toContain(use);
    expect(uses).toEqual(["use std::error::Error;", expect.stringMatching(/^use laminapdf::\{/)]);
    expect(build).toContain("Box<dyn Error>");
    expect(example.trimEnd().endsWith(build)).toBe(true);
    expect(shown).not.toContain("fn main(");
    expect(shown).not.toContain("PathBuf");
  });

  it("keeps a `use` line by any name it brings in, renamed or in braces", () => {
    const source = [
      "use std::error::Error as E;",
      "use std::path::PathBuf;",
      "use laminapdf::{Builder, PageSize};",
      "/// Builds.",
      "pub(crate) fn build() -> Result<(), Box<dyn E>> { Builder::new(); Ok(()) }",
    ].join("\n");
    expect(rustFirstPdf(source).split("\n").slice(0, 3)).toEqual([
      "use std::error::Error as E;",
      "",
      "use laminapdf::{Builder, PageSize};",
    ]);
  });

  it("shows the CLI README's commands that succeed, without `echo $?`", () => {
    const readme = read("crates/laminapdf-cli/README.md");
    const commands = cliCommands().split("\n");
    expect(commands.length).toBeGreaterThan(3);
    for (const command of commands) {
      const [head = "", comment] = command.split("  #");
      expect(readme).toContain(head.trimEnd());
      if (comment !== undefined) expect(readme).toContain(`#${comment}`);
    }
    expect(commands.some((command) => command.includes("echo $?"))).toBe(false);
  });

  it("leaves out a command whose example output is an error", () => {
    const readme = [
      "```console",
      "$ ok a",
      "fine",
      "$ bad b",
      "error[overwrite-refused]: no",
      "$ ok c   # note",
      "```",
    ];
    expect(cliCommands(readme.join("\n")).split("\n")).toEqual(["$ ok a", "$ ok c  # note"]);
    expect(cliCommands()).not.toContain("--in-place");
  });
});

describe("the packages", () => {
  const page = read("apps/docs/src/content/docs/install.mdx");

  it("lists every row of the docs site's install table, in its order", () => {
    const rows = page.split("\n").filter((line) => /^\| `[^`]+` \|/.test(line));
    expect(packages().map((item) => item.name)).toEqual(rows.map((row) => row.match(/`([^`]+)`/)?.[1]));
    expect(packages().map((item) => item.name)).toEqual(expect.arrayContaining(["@laminapdf/sdk", "laminapdf"]));
  });

  it("repeats the install page's note on alpha releases word for word", () => {
    expect(alphaNote()).toMatch(/^Both are alpha releases/);
    expect(page).toContain(alphaNote());
    expect(() => alphaNote("no note")).toThrow('no "Both are alpha releases" note');
  });

  it("shows the install page's two install lines at the published version", () => {
    expect(installLines()).toEqual([`npm install @laminapdf/sdk@${VERSION}`, `cargo add laminapdf@=${VERSION}`]);
    expect(VERSION).toBe(JSON.parse(read("packages/wasm/package.json")).version);
  });
});

describe("the rounds' plain names", () => {
  it("name every section of the board in the snapshot", () => {
    for (const round of progress.rounds) expect(roundName(round.letter)).not.toBe("");
    expect(() => roundName("Z")).toThrow("no plain name");
  });
});
