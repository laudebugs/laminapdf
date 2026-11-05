// The code panel's colours: the highlighter keeps every character and colours the kinds it should,
// the panel renders them as classed spans with no inline style, and site.css colours each kind
// with its --paper-code-* token.

import { readFileSync } from "node:fs";
import { experimental_AstroContainer as AstroContainer } from "astro/container";
import { describe, expect, it } from "vitest";
import CodePanel from "../src/components/home/CodePanel.astro";
import { type CodeKind, highlight } from "../src/data/highlight.js";

const KINDS: readonly CodeKind[] = ["keyword", "string", "comment", "function", "constant"];

const TS = 'import { init } from "@laminapdf/sdk";\n// Load the module once.\nawait init();\nconst pages = 2;';

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
  it("keeps every character of the code, line by line", async () => {
    const lines = await highlight(TS, "ts");
    expect(lines.map((line) => line.map((run) => run.text).join("")).join("\n")).toBe(TS);
  });

  it("colours keywords, strings, comments, functions and constants", async () => {
    const lines = await highlight(TS, "ts");
    const kindOf = (line: number, text: string) => lines[line]?.find((run) => run.text.trim() === text)?.kind;
    expect(kindOf(0, "import")).toBe("keyword");
    expect(kindOf(0, '"@laminapdf/sdk"')).toBe("string");
    expect(kindOf(1, "// Load the module once.")).toBe("comment");
    expect(kindOf(2, "init")).toBe("function");
    expect(kindOf(3, "2")).toBe("constant");
  });

  it("joins neighbouring runs of the same kind", async () => {
    for (const line of await highlight(TS, "ts")) {
      for (const [index, run] of line.entries()) {
        if (index > 0) expect(run.kind).not.toBe(line[index - 1]?.kind);
      }
    }
  });
});

describe("the code panel", () => {
  it("renders each pane as coloured spans with no inline style, and the code unchanged", async () => {
    const container = await AstroContainer.create();
    const tabs = [
      { id: "typescript", label: "TypeScript", code: TS, lang: "ts" },
      { id: "cli", label: "Command line", code: "$ laminapdf info report.pdf  # the facts", lang: "shellsession" },
    ] as const;
    const html = await container.renderToString(CodePanel, { props: { tabs, caption: "A caption" } });
    expect(html).not.toMatch(/style=/);
    const panes = [...html.matchAll(/<pre tabindex="0"><code>([\s\S]*?)<\/code><\/pre>/g)].map(([, pane = ""]) => pane);
    expect(panes.map(textOf)).toEqual(tabs.map((tab) => tab.code));
    expect(panes[0]).toMatch(/<span class="syntax-keyword">import<\/span>/);
    expect(panes[1]).toMatch(/<span class="syntax-comment"># the facts<\/span>/);
  });
});

describe("site.css", () => {
  const css = readFileSync(new URL("../src/styles/site.css", import.meta.url), "utf8");

  it("colours each kind with its own code token", () => {
    for (const kind of KINDS) {
      expect(css).toMatch(new RegExp(`\\.syntax-${kind} \\{\\n  color: var\\(--paper-code-${kind}\\);\\n\\}`));
    }
  });
});
