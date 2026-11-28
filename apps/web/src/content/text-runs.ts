// What Edit mode knows of the text a page already shows (Paper A04), apart from the DOM so Node can
// test it: which run of text a click lands on, how a pick follows its runs through an edit (every
// edit renames them), what the engine says an edit would do, and the two ops that change text. The
// core edits and styles whole runs (a line, or a segment of one in a single font and size), so
// nothing here names part of a run.

import type { ContentTextRun, Op, TextEditReport, TextRunRef, TextStyle, TextSubstitution } from "@laminapdf/sdk";
import type { Box, Point } from "./geometry.ts";

/** How far outside a run's box a click still lands on it, in points. */
const SLOP = 1.5;

/** Part of a new run's box that must lie inside an old run's for it to stand in for that run after an edit. */
const SHARE = 0.4;

/** A run's box in page space: left, top, right, bottom. */
export function runBox(run: ContentTextRun): Box {
  const [x0, y0, x1, y1] = run.bounds;
  return [Math.min(x0, x1), Math.min(y0, y1), Math.max(x0, x1), Math.max(y0, y1)];
}

/** Whether the engine can edit `run` in place. */
export function isEditable(run: ContentTextRun): boolean {
  return run.readOnly === undefined || run.readOnly === null;
}

/** Whether `a` and `b` are the same run: the same text object and the same glyphs. */
export function sameRun(a: ContentTextRun, b: ContentTextRun): boolean {
  return (
    a.run.object.ordinal === b.run.object.ordinal &&
    a.run.object.hash === b.run.object.hash &&
    a.run.start === b.run.start &&
    a.run.end === b.run.end
  );
}

/** The area of `box`. */
function area([x0, y0, x1, y1]: Box): number {
  return Math.max(0, x1 - x0) * Math.max(0, y1 - y0);
}

/** The area `a` and `b` share. */
function shared(a: Box, b: Box): number {
  return area([Math.max(a[0], b[0]), Math.max(a[1], b[1]), Math.min(a[2], b[2]), Math.min(a[3], b[3])]);
}

/**
 * The run under `point`: of those whose box (with a little slop) holds it, the smallest, and the
 * one drawn last when two are alike, so a word in a table cell wins over the cell's whole line.
 * Runs with no area (an empty string) take no click.
 */
export function hitRun(runs: readonly ContentTextRun[], point: Point): ContentTextRun | undefined {
  let best: { run: ContentTextRun; size: number } | undefined;
  for (const run of runs) {
    const box = runBox(run);
    if (area(box) === 0) continue;
    const inside =
      point.x >= box[0] - SLOP && point.x <= box[2] + SLOP && point.y >= box[1] - SLOP && point.y <= box[3] + SLOP;
    if (!inside) continue;
    const size = area(box);
    if (best === undefined || size <= best.size) best = { run, size };
  }
  return best?.run;
}

/** The runs whose centre lies in `box`, in the order the page draws them. */
export function runsWithin(runs: readonly ContentTextRun[], box: Box): ContentTextRun[] {
  return runs.filter((run) => {
    const [x0, y0, x1, y1] = runBox(run);
    const cx = (x0 + x1) / 2;
    const cy = (y0 + y1) / 2;
    return cx >= box[0] && cx <= box[2] && cy >= box[1] && cy <= box[3];
  });
}

/**
 * The runs of `after` that stand where `before` stood: an edit renames the runs it touches and a
 * longer text or another font can split one into several, so each new run that lies mostly inside
 * an old run's box (or, for a style that changes the size, whose box holds that run's centre) is
 * the old run, still. Nothing is left standing for a deleted run.
 */
export function followRuns(before: readonly ContentTextRun[], after: readonly ContentTextRun[]): ContentTextRun[] {
  const found = new Set<ContentTextRun>();
  for (const old of before) {
    const was = runBox(old);
    const centre = { x: (was[0] + was[2]) / 2, y: (was[1] + was[3]) / 2 };
    for (const run of after) {
      const now = runBox(run);
      const mine = area(now) === 0 ? 0 : shared(was, now) / area(now);
      const holds = centre.x >= now[0] && centre.x <= now[2] && centre.y >= now[1] && centre.y <= now[3];
      if (mine >= SHARE || holds) found.add(run);
    }
  }
  return after.filter((run) => found.has(run));
}

/** The run `step` places after `run` among those that can be edited, in the order the page draws them; wraps. */
export function cycleRuns(
  runs: readonly ContentTextRun[],
  current: ContentTextRun | undefined,
  step: 1 | -1,
): ContentTextRun | undefined {
  const editable = runs.filter(isEditable);
  if (editable.length === 0) return undefined;
  const at = current === undefined ? -1 : editable.findIndex((run) => sameRun(run, current));
  if (at < 0) return step > 0 ? editable[0] : editable[editable.length - 1];
  return editable[(at + step + editable.length) % editable.length];
}

/** A font's name as a person reads it: the subset tag off the front and a space for each hyphen. */
export function readableFont(name: string): string {
  return name.replace(/^[A-Z]{6}\+/, "").replaceAll("-", " ");
}

/** What a font's name says of its weight and slant. */
export function fontFacts(name: string): { readonly bold: boolean; readonly italic: boolean } {
  const plain = readableFont(name);
  return {
    bold: /bold|black|heavy|semibold|demibold/i.test(plain),
    italic: /italic|oblique/i.test(plain),
  };
}

/** Which of the registry's standard fonts is nearest the look of the font called `name`. */
export function standardFor(name: string): "helvetica" | "times" | "courier" {
  const plain = readableFont(name);
  if (/courier|mono|consolas|menlo/i.test(plain)) return "courier";
  if (/times|serif|georgia|garamond|palatino|book|newsreader|roman|cambria/i.test(plain) && !/sans/i.test(plain))
    return "times";
  return "helvetica";
}

/** The CSS font stack that draws text like the font called `name`, for the box typed into. */
export function cssFamilyOf(name: string): string {
  switch (standardFor(name)) {
    case "times":
      return '"Times New Roman", Newsreader, serif';
    case "courier":
      return '"Courier New", monospace';
    default:
      return "Helvetica, Arial, sans-serif";
  }
}

/** What the inspector changes about the picked runs. */
export interface StyleChange {
  readonly font?: string;
  readonly size?: number;
  readonly color?: readonly [number, number, number];
  readonly bold?: boolean;
  readonly italic?: boolean;
  readonly underline?: true;
  readonly align?: "left" | "center" | "right";
}

/** The step a change makes, by its one field: the name of its label. */
export function styleStep(
  change: StyleChange,
): "font" | "size" | "colour" | "bold" | "unbold" | "italic" | "unitalic" | "underline" | "align" | "style" {
  const named = Object.keys(change).filter((k) => change[k as keyof StyleChange] !== undefined);
  if (named.length !== 1) return "style";
  switch (named[0]) {
    case "font":
      return "font";
    case "size":
      return "size";
    case "color":
      return "colour";
    case "bold":
      return change.bold === true ? "bold" : "unbold";
    case "italic":
      return change.italic === true ? "italic" : "unitalic";
    case "underline":
      return "underline";
    case "align":
      return "align";
    default:
      return "style";
  }
}

/** The `setTextStyle` fields for `change`, with only what it names. */
export function styleOf(change: StyleChange): TextStyle {
  return {
    ...(change.font === undefined ? {} : { font: change.font }),
    ...(change.size === undefined ? {} : { size: change.size }),
    ...(change.color === undefined ? {} : { color: [...change.color] }),
    ...(change.bold === undefined ? {} : { bold: change.bold }),
    ...(change.italic === undefined ? {} : { italic: change.italic }),
    ...(change.underline === undefined ? {} : { underline: true }),
    ...(change.align === undefined ? {} : { align: change.align }),
  } as unknown as TextStyle;
}

/** The `editText` op that makes `run` show `text`; `fonts` lets characters its font lacks come from other fonts. */
export function editTextOp(page: string, run: TextRunRef, text: string, fonts?: readonly string[]): Op {
  return {
    op: "editText",
    args: { page, run, text, ...(fonts === undefined || fonts.length === 0 ? {} : { fonts: [...fonts] }) },
  } as unknown as Op;
}

/** The `setTextStyle` op that restyles `runs`. */
export function setTextStyleOp(page: string, runs: readonly TextRunRef[], style: TextStyle): Op {
  return { op: "setTextStyle", args: { page, runs: [...runs], ...style } } as unknown as Op;
}

/**
 * The runs of one pick, last drawn first, which is the order that edits of several runs of one
 * page are made in: a change to a later object or later glyphs doesn't renumber an earlier one.
 */
export function lastFirst(runs: readonly ContentTextRun[]): ContentTextRun[] {
  return [...runs].sort(
    (a, b) => b.run.object.ordinal - a.run.object.ordinal || b.run.start - a.run.start || b.run.end - a.run.end,
  );
}

/** What the engine says of an edit before it is made. */
export type TextPlan =
  /** The run's own font draws every character: nothing is said. */
  | { readonly kind: "ok"; readonly report: TextEditReport }
  /** Some characters come from other fonts, which `fonts` names for the op. */
  | {
      readonly kind: "substituted";
      readonly fonts: readonly string[];
      readonly substitutions: readonly TextSubstitution[];
      readonly report: TextEditReport;
    }
  /** No font draws these characters. */
  | { readonly kind: "missing"; readonly characters: readonly string[] }
  /** The text can't be edited in place, for this reason (the engine's word for it). */
  | { readonly kind: "refused"; readonly reason: string };

/** What planning an edit asks of the app. */
export interface PlanHost {
  /** The engine's plan for `run` showing `text`, with `fonts` allowed for the characters it lacks. */
  readonly plan: (page: number, run: TextRunRef, text: string, fonts?: readonly string[]) => Promise<TextEditReport>;
  /** The fonts for `text` set like `primary`, named for an op, and the characters none of them draws. */
  readonly chain: (
    primary: string,
    text: string,
  ) => Promise<{ readonly fonts: readonly string[]; readonly missing: readonly string[] }>;
}

/** The code of an error: its own, or the one its message names. */
export function errorCode(error: unknown): string {
  const code = (error as { code?: unknown } | null)?.code;
  if (typeof code === "string") return code;
  const found = error instanceof Error ? /(missing-glyph|content-[a-z-]+)/.exec(error.message) : null;
  return found?.[0] ?? "";
}

/** The engine's reasons a run can't be edited. */
export const REASONS = [
  "type3-font",
  "vertical-text",
  "no-unicode-map",
  "rotated-run",
  "glyphs-out-of-order",
  "actual-text",
  "right-to-left",
  "too-deep",
  "content-unreadable",
  "clipping-path",
  "state-not-portable",
] as const;

/** The reason an error gives for a refusal: its `reason` field, or one of the known reasons its message names. */
export function reasonOf(error: unknown): string {
  const data = (error as { data?: { reason?: unknown } } | null)?.data;
  if (typeof data?.reason === "string") return data.reason;
  const message = error instanceof Error ? error.message : "";
  return REASONS.find((reason) => message.includes(reason)) ?? message;
}

/** The character a `missing-glyph` error names ("U+00E8"), if its message does. */
export function missingOf(error: unknown): string | undefined {
  const found = error instanceof Error ? /U\+([0-9A-F]{4,6})/i.exec(error.message) : null;
  return found?.[1] === undefined ? undefined : String.fromCodePoint(Number.parseInt(found[1], 16));
}

/**
 * What changing `run` to `text` would do. The run's own font goes first; only if it lacks a
 * character is the chain of other fonts planned, so a font is never changed unless the person has
 * been told: the plan names the characters and the fonts that would draw them.
 */
export async function planText(host: PlanHost, page: number, run: ContentTextRun, text: string): Promise<TextPlan> {
  if (!isEditable(run)) return { kind: "refused", reason: String(run.readOnly) };
  try {
    const report = await host.plan(page, run.run, text);
    return report.substitutions.length === 0 ? { kind: "ok", report } : substituted([], report);
  } catch (error) {
    if (errorCode(error) !== "missing-glyph") return { kind: "refused", reason: reasonOf(error) };
    return withOthers(host, page, run, text, missingOf(error));
  }
}

/** A plan with fonts for the characters the run's own lacks. */
function substituted(fonts: readonly string[], report: TextEditReport): TextPlan {
  return { kind: "substituted", fonts, substitutions: report.substitutions, report };
}

/** `planText` once the run's own font fell short: plans again with the chain of fonts the registry offers. */
async function withOthers(
  host: PlanHost,
  page: number,
  run: ContentTextRun,
  text: string,
  first: string | undefined,
): Promise<TextPlan> {
  const chain = await host.chain(standardFor(run.fontName), text);
  // A character no registry font draws may still be one the run's own font does, so the plan decides; its words come after.
  try {
    const report = await host.plan(page, run.run, text, chain.fonts);
    return substituted(chain.fonts, report);
  } catch (error) {
    if (errorCode(error) === "missing-glyph") {
      const named = chain.missing.length > 0 ? chain.missing : [missingOf(error) ?? first ?? "?"];
      return { kind: "missing", characters: named };
    }
    return { kind: "refused", reason: reasonOf(error) };
  }
}

/** The characters of a plan's substitutions, each once, in order. */
export function substitutedCharacters(substitutions: readonly TextSubstitution[]): string[] {
  return [...new Set(substitutions.flatMap((s) => [...s.text].filter((c) => c.trim() !== "")))];
}

/** The font that draws most of a plan's substituted characters, as the engine names it. */
export function substituteFont(substitutions: readonly TextSubstitution[]): string | undefined {
  const counts = new Map<string, number>();
  for (const s of substitutions) counts.set(s.font, (counts.get(s.font) ?? 0) + [...s.text].length);
  let best: string | undefined;
  let most = 0;
  for (const [font, count] of counts) {
    if (count > most) {
      best = font;
      most = count;
    }
  }
  return best;
}
