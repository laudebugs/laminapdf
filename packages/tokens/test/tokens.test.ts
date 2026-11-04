import { describe, expect, it } from "vitest";
import { colour, cssName, emitContrast, emitCss, emitTs } from "../scripts/emit.ts";
import {
  ACCENTS,
  APPEARANCES,
  aliasOf,
  appearanceContext,
  type Colour,
  type Context,
  contexts,
  flatten,
  measure,
  resolve,
  themeContext,
} from "../scripts/model.ts";
import { ANNOTATION_COLOURS, cssVar, THEMES } from "../src/index.ts";
import { loadTokens, readRepo } from "./helpers.ts";

const tokens = loadTokens();
const css = readRepo("packages/tokens/src/generated/tokens.css");

/** The custom properties each top-level block of the CSS declares, keyed by its selector. */
function cssBlocks(text: string): { selector: string; names: string[] }[] {
  return [...text.matchAll(/^ *(:where\([^\n{]+\)) \{([^}]*)\}/gm)].map((m) => ({
    selector: m[1] ?? "",
    names: [...(m[2] ?? "").matchAll(/--paper-([\w-]+):/g)].map((n) => n[1] ?? ""),
  }));
}

/** What a context's colours lack: a role, the pairs their role needs, or a pair measured on them. */
function roleProblems(ctx: Context, accent: string): string[] {
  const backgrounds = new Set(measure(tokens, ctx, accent).map((p) => p.on));
  const needsPairs = new Set(["text", "boundary", "focus"]);
  return [...ctx.names].flatMap(([name, path]) => {
    const token = tokens.get(path);
    if (token?.type !== "color") return [];
    const { role, contrast = [] } = token.lamina;
    if (role === undefined) return [`${ctx.label}: ${name} has no role`];
    if (needsPairs.has(role) && contrast.length === 0) return [`${ctx.label}: ${name} is ${role} but declares no pair`];
    if (role === "background" && !backgrounds.has(name)) return [`${ctx.label}: nothing is measured on ${name}`];
    return [];
  });
}

describe("tokens.json", () => {
  it("resolves every alias, and refuses unknown references and cycles", () => {
    for (const path of tokens.keys()) expect(() => resolve(tokens, path)).not.toThrow();
    const meta = { "dev.lamina": { status: "from-files" } };
    const doc = (a: string, b: string) => ({
      x: { $type: "color", a: { $value: a, $extensions: meta }, b: { $value: b, $extensions: meta } },
    });
    expect(() => resolve(flatten(doc("{x.b}", "{x.a}")), "x.a")).toThrow(/alias cycle: x.a → x.b → x.a/);
    expect(() => resolve(flatten(doc("{x.c}", "{x.a}")), "x.a")).toThrow(/unknown reference \{x.c\}/);
  });

  it("gives every UI theme the same semantic tokens", () => {
    const keys = (theme: string, accent: string) => [...themeContext(tokens, theme, accent).names.keys()].sort();
    const light = keys("light", "teal");
    for (const accent of ACCENTS) {
      expect(keys("light", accent)).toEqual(light);
      expect(keys("dark", accent)).toEqual(light);
    }
    expect(keys("high-contrast", "teal")).toEqual(light);
  });

  it("gives every page appearance the same tokens", () => {
    const keys = (page: string) => [...appearanceContext(tokens, page, "teal").names.keys()].sort();
    for (const page of APPEARANCES) expect(keys(page)).toEqual(keys("original"));
  });

  it("gives every colour a role, and every role the pairs it needs", () => {
    expect(contexts(tokens).flatMap(({ ctx, accent }) => roleProblems(ctx, accent))).toEqual([]);
  });

  it("writes each colour's hex as its components say", () => {
    const differ = [...tokens.values()]
      .filter((t) => t.type === "color" && aliasOf(t.value) === undefined)
      .filter((t) => (t.value as Colour).hex !== colour({ ...(t.value as Colour), alpha: 1 }))
      .map((t) => t.path);
    expect(differ).toEqual([]);
  });

  it("lets component tokens read semantic tokens, never primitives", () => {
    const direct = [...tokens.values()]
      .filter((t) => t.path.startsWith("component.") && aliasOf(t.value)?.startsWith("primitive."))
      .map((t) => t.path);
    expect(direct).toEqual([]);
  });

  it("has no proposed value left (the owner signs them off before merge)", () => {
    const proposed = [...tokens.values()].filter((t) => t.lamina.status === "proposed").map((t) => t.path);
    expect(proposed).toEqual([]);
  });
});

describe("the design language", () => {
  // Colour names in its prose that aren't values: the canvas Green §2.8 replaced.
  const NOT_TOKENS = new Set(["#3E4A3C"]);

  it("has a token for every colour of §2.1–§2.4 and §2.8", () => {
    const md = readRepo("docs/design/paper/design-language.md");
    const section = (from: string, to: string) => md.slice(md.indexOf(from), md.indexOf(to, md.indexOf(from)));
    const text = section("### 2.1", "### 2.5") + section("### 2.8", "## 3.");
    const wanted = new Set<string>();
    for (const line of text.split("\n")) {
      if (line.startsWith("| `window`")) continue; // the drawn macOS frame: mockups only
      // "(= `#EAE7E1` on ground)" is a composited result, not a value.
      for (const m of line.matchAll(/(= `)?(#[0-9A-Fa-f]{6})\b/g))
        if (m[1] === undefined) wanted.add(m[2]?.toUpperCase() ?? "");
    }
    const have = new Set(readRepo("packages/tokens/tokens.json").match(/#[0-9A-F]{6}/g));
    const missing = [...wanted].filter((h) => !have.has(h) && !NOT_TOKENS.has(h));
    expect(missing).toEqual([]);
  });
});

describe("generated output", () => {
  it("is what the generator writes now", () => {
    expect(css).toBe(emitCss(tokens));
    expect(readRepo("packages/tokens/src/generated/tokens.ts")).toBe(emitTs(tokens));
    expect(readRepo("packages/tokens/CONTRAST.md")).toBe(emitContrast(tokens));
  });

  it("declares the annotation colours once, outside every theme and appearance", () => {
    const blocks = cssBlocks(css);
    const declaring = blocks.filter((b) => b.names.some((n) => n.startsWith("annotation-")));
    expect(declaring.map((b) => b.selector)).toEqual([":where(:root)"]);
    for (const [name, value] of Object.entries(ANNOTATION_COLOURS)) {
      expect(css.match(new RegExp(`--paper-annotation-${name}:`, "g"))).toHaveLength(1);
      expect(css).toContain(`--paper-annotation-${name}: ${value};`);
    }
  });

  it("names every theme token once per theme block, never a primitive", () => {
    for (const block of cssBlocks(css)) expect(new Set(block.names).size, block.selector).toBe(block.names.length);
    expect(css).not.toContain("--paper-primitive");
    for (const path of tokens.keys()) if (path.startsWith("primitive.")) expect(cssName(path)).toBeUndefined();
  });

  it("applies each accent's dark values wherever the dark theme applies", () => {
    for (const accent of ACCENTS.filter((a) => a !== "teal")) {
      expect(css).toContain(`:where([data-theme="dark"][data-accent="${accent}"]) {`);
      expect(css).toContain(`  :where(:root:not([data-theme])[data-accent="${accent}"]) {`);
    }
    expect(css).toContain("@media (forced-colors: active) {");
  });

  it("types the constants and the CSS reference", () => {
    expect(THEMES.dark.blue.accent).toBe("#91B3DC");
    expect(THEMES["high-contrast"].plum.accent).toBe("LinkText");
    expect(cssVar("surface")).toBe("var(--paper-surface)");
  });
});
