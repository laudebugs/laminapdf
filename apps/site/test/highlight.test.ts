// The code panel's colours, on the home page's own code: the highlighter keeps every character and
// gives each language's keywords, strings, comments, names and numbers their kinds; the opening
// renders the panes as classed spans with no inline style; and site.css colours each kind with its
// --paper-code-* token.

import { readFileSync } from "node:fs";
import { CODE_KINDS, type CodeKind, type CodeToken } from "@laminapdf/site-kit/code-kinds";
import { experimental_AstroContainer as AstroContainer } from "astro/container";
import { describe, expect, it } from "vitest";
import Opening from "../src/components/home/Opening.astro";
import { type CodeLanguage, highlight } from "../src/data/highlight.js";
import { cliCommands, rustFirstPdf, typescriptFirstPdf } from "../src/data/sources.js";

/** The panes' code and language, in the order the opening shows them. */
const PANES: readonly (readonly [lang: CodeLanguage, code: string])[] = [
  ["ts", typescriptFirstPdf()],
  ["rust", rustFirstPdf()],
  ["console", cliCommands()],
];

/** Every run of `lines` whose text, trimmed, is `text`. */
function runs(lines: readonly CodeToken[][], text: string): CodeToken[] {
  return lines.flat().filter((run) => run.text.trim() === text);
}

/** The kinds of the runs of `lines` whose text, trimmed, is `text`; `undefined` for a plain one. */
function kinds(lines: readonly CodeToken[][], text: string): (CodeKind | undefined)[] {
  const found = runs(lines, text);
  expect(found, `no run is "${text}"`).not.toEqual([]);
  return found.map((run) => run.kind);
}

/** The coloured runs of `lines` of kind `kind`. */
function ofKind(lines: readonly CodeToken[][], kind: CodeKind): string[] {
  return lines.flat().flatMap((run) => (run.kind === kind ? [run.text] : []));
}

/** `html` with its tags removed and its entities decoded. */
function textOf(html: string): string {
  return html
    .replace(/<[^>]+>/g, "")
    .replaceAll("&quot;", '"')
    .replaceAll("&#39;", "'")
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
    .replaceAll("&amp;", "&");
}

describe("the highlighter", () => {
  it("keeps every character of each pane, line by line, and joins neighbouring runs of one kind", async () => {
    for (const [lang, code] of PANES) {
      const lines = await highlight(code, lang);
      expect(lines.map((line) => line.map((run) => run.text).join("")).join("\n")).toBe(code);
      for (const line of lines) {
        for (const [index, run] of line.entries()) if (index > 0) expect(run.kind).not.toBe(line[index - 1]?.kind);
      }
    }
  });

  it("colours TypeScript's keywords, strings, comments, function names, types and numbers", async () => {
    const lines = await highlight(typescriptFirstPdf(), "ts");
    for (const keyword of ["import", "export", "function", "using", "const", "new", "return"]) {
      expect(kinds(lines, keyword)).toContain("keyword");
    }
    expect(kinds(lines, '"@laminapdf/sdk"')).toEqual(["string"]);
    expect(ofKind(lines, "comment").some((text) => text.startsWith("// Page space"))).toBe(true);
    for (const name of ["hello", "page", "surface", "text", "finish", "save"]) {
      expect(kinds(lines, name)).toContain("function");
    }
    for (const constant of ["Uint8Array", "ArrayBuffer", "24", "72"]) {
      expect(kinds(lines, constant)).toContain("constant");
    }
  });

  it("colours Rust's keywords, strings, comments, function names, types and numbers", async () => {
    const lines = await highlight(rustFirstPdf(), "rust");
    for (const keyword of ["use", "pub", "fn", "let", "mut"]) expect(kinds(lines, keyword)).toContain("keyword");
    expect(kinds(lines, '"Hello, world!"')).toEqual(["string"]);
    expect(ofKind(lines, "comment").some((text) => text.startsWith("/// The document's bytes"))).toBe(true);
    for (const name of ["build", "standard_font", "page", "finish", "save"]) {
      expect(kinds(lines, name)).toContain("function");
    }
    for (const constant of ["BuildOptions", "Result", "u8", "24.0"]) {
      expect(kinds(lines, constant)).toContain("constant");
    }
  });

  it("leaves operators, punctuation and variables plain", async () => {
    for (const [lang, code] of PANES.filter(([lang]) => lang !== "console")) {
      const lines = await highlight(code, lang);
      for (const keyword of ofKind(lines, "keyword")) expect(keyword, `${lang}: a keyword run`).toMatch(/^[a-z]+$/);
      for (const name of ofKind(lines, "function")) expect(name, `${lang}: a function run`).toMatch(/^\w+$/);
      for (const variable of ["b", "doc", "page", "options", "helvetica"]) {
        expect(ofKind(lines, "constant"), `${lang}: ${variable}`).not.toContain(variable);
      }
    }
  });

  it("colours a console's prompts, commands, options, quoted strings and comments, not its arguments", async () => {
    const lines = await highlight(cliCommands(), "console");
    for (const line of lines) {
      expect(line[0]).toEqual({ text: "$ ", kind: "comment" });
      expect(line[1]?.kind).toBe("function");
    }
    expect(kinds(lines, "laminapdf").every((kind) => kind === "function")).toBe(true);
    for (const option of ["--json", "--fields", "-o"]) expect(kinds(lines, option)).toContain("constant");
    expect(ofKind(lines, "string")).toContain(`'select(.type=="result" and .status!="ok")'`);
    expect(ofKind(lines, "comment").some((text) => text.startsWith("# read-only"))).toBe(true);
    for (const argument of ["report.pdf", "invoice.pdf", "xref-repaired"]) {
      expect(lines.flat().some((run) => run.kind === undefined && run.text.includes(argument))).toBe(true);
    }
  });

  it("leaves a console line without a prompt plain, as output", async () => {
    expect(await highlight("$ laminapdf info a.pdf\na.pdf: 3 pages", "console")).toEqual([
      [{ text: "$ ", kind: "comment" }, { text: "laminapdf", kind: "function" }, { text: " info a.pdf" }],
      [{ text: "a.pdf: 3 pages" }],
    ]);
  });
});

describe("the opening's code panel", () => {
  it("renders each pane as coloured spans with no inline style, and the code unchanged", async () => {
    const container = await AstroContainer.create();
    const html = await container.renderToString(Opening, { props: { date: "2026-11-05" } });
    expect(html).not.toMatch(/style=/);
    const panes = [...html.matchAll(/<pre tabindex="0"><code>([\s\S]*?)<\/code><\/pre>/g)].map(([, pane = ""]) => pane);
    expect(panes.map(textOf)).toEqual(PANES.map(([, code]) => code));
    const coloured = (pane: string) =>
      new Set([...pane.matchAll(/<span class="syntax-([a-z]+)">/g)].map(([, kind]) => kind));
    expect(coloured(panes[0] ?? "")).toEqual(new Set(CODE_KINDS));
    expect(coloured(panes[1] ?? "")).toEqual(new Set(CODE_KINDS));
    expect(coloured(panes[2] ?? "")).toEqual(new Set(["comment", "function", "constant", "string"]));
  });
});

describe("site.css", () => {
  const css = readFileSync(new URL("../src/styles/site.css", import.meta.url), "utf8");

  it("colours each kind with its own code token", () => {
    for (const kind of CODE_KINDS) {
      expect(css).toMatch(new RegExp(`\\.syntax-${kind}\\s*\\{[^}]*\\bcolor:\\s*var\\(--paper-code-${kind}\\)`));
    }
  });
});
