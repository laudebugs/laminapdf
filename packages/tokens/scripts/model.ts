// tokens.json as data the generator and the tests share: the tokens flattened by path, aliases
// resolved (unknown references and cycles refused), the contexts a contrast pair is read in, and
// the CSS name and value of each token. Node-free, so the tests import it as it is.

import { composite, contrastRatio, type Rgba } from "../src/contrast.ts";

/** Where a value came from: copied from the design files, proposed, or approved by the owner. */
type Status = "from-files" | "proposed" | "approved";

/** What a colour does, which decides the pairs it needs (README: "Declaring a pair"). */
type Role = "background" | "text" | "boundary" | "focus" | "decorative";

/** A foreground on a background, measured against `min`; `{x}` names a token of the same context. */
interface Pair {
  on: string;
  min: number;
  fg?: string;
  over?: string;
  kind?: "text" | "boundary" | "focus";
  decorative?: boolean;
  exempt?: string;
}

/** The `$extensions["dev.lamina"]` entry every token above the primitives carries. */
interface Lamina {
  /** Absent on primitives: the tokens that use one carry the status of the value. */
  status?: Status;
  role?: Role;
  contrast?: Pair[];
  system?: string;
  followsAccent?: boolean;
  reducedMotion?: Dimension;
}

/** A DTCG dimension or duration. */
export interface Dimension {
  value: number;
  unit: string;
}

/** A DTCG sRGB colour. */
export interface Colour {
  colorSpace: "srgb";
  components: [number, number, number];
  alpha?: number;
  hex?: string;
}

/** One token: its dotted path, its `$type` (inherited from the closest group), value and metadata. */
export interface Token {
  path: string;
  type: string;
  value: unknown;
  description: string;
  lamina: Lamina;
}

export const THEMES = ["light", "dark", "high-contrast"] as const;
export const ACCENTS = ["teal", "blue", "plum", "green"] as const;
export const APPEARANCES = ["original", "sepia", "dark", "high-contrast"] as const;
/** The accent a `followsAccent` token names in tokens.json; the generator swaps in each one. */
export const DEFAULT_ACCENT = "teal";

type Json = { [key: string]: unknown };

const isObject = (v: unknown): v is Json => typeof v === "object" && v !== null && !Array.isArray(v);

/** Every token in `doc`, by dotted path, in file order. */
export function flatten(doc: Json): Map<string, Token> {
  const out = new Map<string, Token>();
  const walk = (node: Json, path: string[], type: string | undefined) => {
    const own = typeof node.$type === "string" ? node.$type : type;
    if ("$value" in node) {
      const token = toToken(node, path.join("."), own);
      out.set(token.path, token);
      return;
    }
    for (const [name, child] of Object.entries(node)) {
      if (!name.startsWith("$") && isObject(child)) walk(child, [...path, name], own);
    }
  };
  walk(doc, [], undefined);
  return out;
}

/** One `$value` node as a token; every token above the primitives must carry a status. */
function toToken(node: Json, path: string, type: string | undefined): Token {
  if (type === undefined) throw new Error(`${path}: no $type on the token or a group above it`);
  const ext = isObject(node.$extensions) ? node.$extensions["dev.lamina"] : {};
  if (!isObject(ext) || (!path.startsWith("primitive.") && typeof ext.status !== "string")) {
    throw new Error(`${path}: no $extensions["dev.lamina"].status`);
  }
  const description = typeof node.$description === "string" ? node.$description : "";
  return { path, type, value: node.$value, description, lamina: ext as unknown as Lamina };
}

const ALIAS = /^\{([^{}]+)\}$/;

/** The path an alias names, or undefined when `value` isn't one. */
export function aliasOf(value: unknown): string | undefined {
  return typeof value === "string" ? ALIAS.exec(value)?.[1] : undefined;
}

/** The token at the end of `path`'s alias chain; throws on an unknown reference or a cycle. */
export function resolve(tokens: Map<string, Token>, path: string, seen: string[] = []): Token {
  if (seen.includes(path)) throw new Error(`alias cycle: ${[...seen, path].join(" → ")}`);
  const token = tokens.get(path);
  if (token === undefined) {
    throw new Error(seen.length > 0 ? `${seen.at(-1)}: unknown reference {${path}}` : `unknown token ${path}`);
  }
  const target = aliasOf(token.value);
  return target === undefined ? token : resolve(tokens, target, [...seen, path]);
}

/** A set of short names (`ground`, `ink`) and the token paths they stand for in one combination. */
export interface Context {
  label: string;
  names: Map<string, string>;
}

/** The semantic colours of a UI theme with an accent (high contrast ignores the accent). */
export function themeContext(tokens: Map<string, Token>, theme: string, accent: string): Context {
  const names = new Map<string, string>();
  const groups = theme === "high-contrast" ? [`theme.${theme}.`] : [`theme.${theme}.`, `accent.${accent}.${theme}.`];
  for (const path of tokens.keys()) {
    for (const prefix of groups) if (path.startsWith(prefix)) names.set(path.slice(prefix.length), path);
  }
  const label = theme === "high-contrast" ? theme : `${theme} · ${accent}`;
  return { label, names };
}

/** A page appearance's colours with an accent, `followsAccent` tokens pointed at that accent. */
export function appearanceContext(tokens: Map<string, Token>, appearance: string, accent: string): Context {
  const prefix = `appearance.${appearance}.`;
  const names = new Map<string, string>();
  for (const path of tokens.keys()) if (path.startsWith(prefix)) names.set(path.slice(prefix.length), path);
  const label = appearance === "high-contrast" ? `page ${appearance}` : `page ${appearance} · ${accent}`;
  return { label, names };
}

/** The token a contrast reference names: a short name of the context, else a full path. */
function lookup(tokens: Map<string, Token>, ctx: Context, accent: string, ref: string): Token {
  const name = aliasOf(ref);
  if (name === undefined) throw new Error(`${ctx.label}: "${ref}" is not a {reference}`);
  const path = ctx.names.get(name) ?? name;
  return resolveFor(tokens, path, accent);
}

/** `resolve`, with a `followsAccent` token's default accent replaced by `accent`. */
export function resolveFor(tokens: Map<string, Token>, path: string, accent: string): Token {
  const token = tokens.get(path);
  if (!token?.lamina.followsAccent) return resolve(tokens, path);
  const target = aliasOf(token.value);
  if (!target?.startsWith(`accent.${DEFAULT_ACCENT}.page.`)) {
    throw new Error(`${path}: followsAccent needs an alias to accent.${DEFAULT_ACCENT}.page.…`);
  }
  return resolve(tokens, swapAccent(target, accent), [path]);
}

/** `accent.teal.…` as `accent.<accent>.…`. */
function swapAccent(path: string, accent: string): string {
  const prefix = `accent.${DEFAULT_ACCENT}.`;
  return path.startsWith(prefix) ? `accent.${accent}.${path.slice(prefix.length)}` : path;
}

/** A colour token's value as straight RGBA in 0–255 (alpha 0–1). */
function rgba(token: Token): Rgba {
  const c = token.value as Colour;
  if (token.type !== "color" || c.colorSpace !== "srgb") throw new Error(`${token.path}: not an sRGB colour`);
  const [r, g, b] = c.components.map((x) => Math.round(x * 255)) as [number, number, number];
  return { r, g, b, a: c.alpha ?? 1 };
}

/** One measured pair. `ratio` is undefined when a system colour makes it a browser-only check. */
export interface Measured {
  context: string;
  fg: string;
  on: string;
  ratio: number | undefined;
  min: number;
  kind: "text" | "boundary" | "focus";
  waived: string | undefined;
}

/** Every pair the tokens of `ctx` declare, measured (translucent colours composited first). */
export function measure(tokens: Map<string, Token>, ctx: Context, accent: string): Measured[] {
  return [...ctx.names].flatMap(([short, path]) =>
    (tokens.get(path)?.lamina.contrast ?? []).map((pair) => measurePair(tokens, ctx, accent, short, pair)),
  );
}

function measurePair(tokens: Map<string, Token>, ctx: Context, accent: string, short: string, pair: Pair): Measured {
  const find = (ref: string) => lookup(tokens, ctx, accent, ref);
  const fg = pair.fg === undefined ? find(`{${short}}`) : find(pair.fg);
  const on = find(pair.on);
  const over = pair.over === undefined ? undefined : find(pair.over);
  const system = [fg, on, over].some((t) => t?.lamina.system !== undefined);
  return {
    context: ctx.label,
    fg: pair.fg === undefined ? short : (aliasOf(pair.fg) ?? pair.fg),
    on: aliasOf(pair.on) ?? pair.on,
    ratio: system ? undefined : ratioOf(fg, on, over, `${ctx.label}: ${short} on ${pair.on}`),
    min: pair.min,
    kind: pair.kind ?? "text",
    waived: pair.exempt ?? (pair.decorative ? "decorative" : undefined),
  };
}

function ratioOf(fg: Token, on: Token, over: Token | undefined, what: string): number {
  let bg = rgba(on);
  if (bg.a < 1) {
    if (over === undefined) throw new Error(`${what}: the background is translucent; name what it sits "over"`);
    bg = composite(bg, rgba(over));
  }
  return contrastRatio(composite(rgba(fg), bg), bg);
}

/** Every context the contrast test reads: 2 themes × 4 accents, high contrast, 4 appearances × 4 accents. */
export function contexts(tokens: Map<string, Token>): { ctx: Context; accent: string }[] {
  const out: { ctx: Context; accent: string }[] = [];
  for (const accent of ACCENTS) {
    for (const theme of ["light", "dark"]) out.push({ ctx: themeContext(tokens, theme, accent), accent });
    for (const page of APPEARANCES) {
      if (page !== "high-contrast" || accent === DEFAULT_ACCENT) {
        out.push({ ctx: appearanceContext(tokens, page, accent), accent });
      }
    }
  }
  out.push({ ctx: themeContext(tokens, "high-contrast", DEFAULT_ACCENT), accent: DEFAULT_ACCENT });
  return out;
}

/** The paths of the fixed colours (annotation and content palettes) and the pairs they declare. */
export function fixedContext(tokens: Map<string, Token>): Context {
  const names = new Map<string, string>();
  for (const path of tokens.keys()) {
    if (path.startsWith("annotation.") || path.startsWith("content.")) names.set(path, path);
  }
  return { label: "fixed colours", names };
}
