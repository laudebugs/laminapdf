// The page-setup dialogs' arithmetic (Paper O09): lengths in millimetres or inches, the papers
// Resize offers, what a crop leaves visible, and the sizes a selection's pages have. Every length
// the ops take is in points (1/72 in); the dialogs show millimetres or inches by locale.

/** The unit lengths show in. */
export type LengthUnit = "mm" | "in";

/** Points in one of each unit. */
const POINTS: Readonly<Record<LengthUnit, number>> = { mm: 72 / 25.4, in: 72 };

/** The unit a locale measures paper in: inches in the United States, Liberia and Myanmar, millimetres elsewhere. */
export function unitFor(locale: string): LengthUnit {
  const region = /[-_]([A-Za-z]{2})\b/.exec(locale)?.[1]?.toUpperCase();
  return region === "US" || region === "LR" || region === "MM" ? "in" : "mm";
}

/** `points` in `unit`, rounded as the dialogs show it: whole millimetres, or inches to two places. */
export function shown(points: number, unit: LengthUnit): string {
  const value = points / POINTS[unit];
  return unit === "mm" ? String(Math.round(value)) : String(Math.round(value * 100) / 100);
}

/** A length typed in `unit`, in points; undefined when it isn't a number from 0. */
export function points(text: string, unit: LengthUnit): number | undefined {
  const value = text.trim().replace(",", ".");
  if (!/^\d+(\.\d+)?$|^\.\d+$/.test(value)) return undefined;
  return Number(value) * POINTS[unit];
}

/** One step of the arrow keys on a length field: 1 mm (Shift: 10 mm), or 0.1 in (Shift: 1 in). */
export function step(unit: LengthUnit, large: boolean): number {
  if (unit === "mm") return large ? 10 : 1;
  return large ? 1 : 0.1;
}

/** `text` moved by `delta` in its unit, never below 0. */
export function nudged(text: string, delta: number, unit: LengthUnit): string {
  const now = points(text, unit) ?? 0;
  const next = Math.max(0, now / POINTS[unit] + delta);
  return unit === "mm" ? String(Math.round(next)) : String(Math.round(next * 100) / 100);
}

/** The papers Resize offers, upright, in points. */
export const PAPERS = {
  a4: { width: (210 * 72) / 25.4, height: (297 * 72) / 25.4 },
  letter: { width: 612, height: 792 },
  legal: { width: 612, height: 1008 },
  a5: { width: (148 * 72) / 25.4, height: (210 * 72) / 25.4 },
} as const;

/** A paper Resize offers, or `custom`. */
export type PaperName = keyof typeof PAPERS | "custom";

/** A page's displayed size in points. */
export interface PageSize {
  readonly width: number;
  readonly height: number;
}

/** Within half a millimetre: the same paper. */
const near = (a: number, b: number) => Math.abs(a - b) < 72 / 25.4 / 2;

/** The paper `size` is, either way up, or undefined. */
export function paperOf(size: PageSize): keyof typeof PAPERS | undefined {
  for (const [name, paper] of Object.entries(PAPERS) as [keyof typeof PAPERS, PageSize][]) {
    const upright = near(size.width, paper.width) && near(size.height, paper.height);
    const turned = near(size.width, paper.height) && near(size.height, paper.width);
    if (upright || turned) return name;
  }
  return undefined;
}

/** The different sizes among `pages`, by value within half a millimetre, in order of first use. */
export function sizesOf(pages: readonly PageSize[]): PageSize[] {
  const out: PageSize[] = [];
  for (const page of pages) {
    if (!out.some((s) => near(s.width, page.width) && near(s.height, page.height))) out.push(page);
  }
  return out;
}

/** `size` the way `orientation` asks: portrait taller than wide, landscape wider than tall. */
export function oriented(size: PageSize, orientation: "portrait" | "landscape"): PageSize {
  const long = Math.max(size.width, size.height);
  const short = Math.min(size.width, size.height);
  return orientation === "portrait" ? { width: short, height: long } : { width: long, height: short };
}

/** Margins in points, as the crop op takes them. */
export interface Margins {
  readonly top: number;
  readonly right: number;
  readonly bottom: number;
  readonly left: number;
}

/** What `margins` leave of `size`, or undefined when they leave nothing. */
export function visibleArea(size: PageSize, margins: Margins): PageSize | undefined {
  const width = size.width - margins.left - margins.right;
  const height = size.height - margins.top - margins.bottom;
  return width > 0 && height > 0 ? { width, height } : undefined;
}
