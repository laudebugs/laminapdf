// What the How we test page shows from proof.json, as people read it: names for the sources, tools,
// standards and runtimes the snapshot keys by code name, percentages that never round up to 100 %,
// and the sentence a section shows in place of its figure on a night whose job left no report.

import { HOW_WE_TEST } from "./copy.js";
import type { Night } from "./proof.js";

/** The corpus sources, in the order the page lists them. */
const SOURCES: Readonly<Record<string, string>> = {
  pdfjs: "pdf.js",
  pdfbox: "PDFBox",
  pdfium: "PDFium",
  qpdf: "qpdf",
  mutants: "Lamina PDF's own files, damaged on purpose",
  all: "All files",
};

/** The other tools, by the snapshot's key. */
const TOOLS: Readonly<Record<string, string>> = {
  pdfjs: "pdf.js",
  pdfium: "PDFium",
  qpdf: "qpdf",
  pypdf: "pypdf",
  pdftotext: "pdftotext",
};

const PROFILES: Readonly<Record<string, string>> = {
  "2b": "PDF/A-2b",
  "2u": "PDF/A-2u",
  "3b": "PDF/A-3b",
  ua1: "PDF/UA-1",
};

const RUNTIMES: Readonly<Record<string, string>> = {
  node: "Node.js",
  deno: "Deno",
  bun: "Bun",
  workerd: "Cloudflare Workers",
  chromium: "Chromium",
  firefox: "Firefox",
  webkit: "WebKit (Safari)",
};

/** The nightly's jobs, as the page names them when one fails. */
const JOBS: Readonly<Record<string, string>> = {
  differential: "the comparison with other tools",
  fuzz: "fuzzing",
  scaling: "the large-file tests",
  "render-diff": "the drawing comparison",
  verapdf: "the PDF/A and PDF/UA checks",
  roundtrip: "the saved-file checks",
  "pdf-runtimes": "the JavaScript runtimes",
};

/** Each section's job, whose result explains a missing section. */
export const SECTION_JOBS = {
  corpus: "differential",
  differential: "differential",
  text: "differential",
  fuzz: "fuzz",
  verapdf: "verapdf",
  roundtrip: "roundtrip",
  runtimes: "pdf-runtimes",
  render: "render-diff",
} as const satisfies Partial<Record<keyof Night, string>>;

/** `key`'s name in `names`, or the key itself for one the page doesn't know yet. */
const named = (names: Readonly<Record<string, string>>, key: string): string => names[key] ?? key;

export const sourceName = (key: string): string => named(SOURCES, key);
export const toolName = (key: string): string => named(TOOLS, key);
export const profileName = (key: string): string => named(PROFILES, key);
export const runtimeName = (key: string): string => named(RUNTIMES, key);
const jobName = (key: string): string => named(JOBS, key);

/** `entries` in the order of `names`' keys, then any key it doesn't know, alphabetically. */
export function ordered<T>(
  entries: Readonly<Record<string, T>>,
  names: "sources" | "tools",
): readonly (readonly [string, T])[] {
  const order = Object.keys(names === "sources" ? SOURCES : TOOLS);
  const rank = (key: string) => (order.includes(key) ? order.indexOf(key) : order.length);
  return Object.entries(entries).sort(([a], [b]) => rank(a) - rank(b) || a.localeCompare(b));
}

const NUMBER = new Intl.NumberFormat("en-GB");

/** `n` with thousands separators: "16,041,418". */
export const figure = (n: number): string => NUMBER.format(n);

/**
 * `part` of `whole` as a percentage with one decimal, "92.2%". It shows 100% only when every one
 * counts and 0% only when none does, so a rounded figure never claims more or less than happened;
 * "–" when there is nothing to count.
 */
export function percent(part: number, whole: number): string {
  if (whole === 0) return "–";
  if (part === whole) return "100%";
  if (part === 0) return "0%";
  const tenths = Math.min(999, Math.max(1, Math.round((part / whole) * 1000)));
  return `${(tenths / 10).toFixed(1)}%`;
}

/** A share in basis points (10,000 = 100 %), as {@link percent} writes it. */
export const basisPoints = (bp: number): string => percent(bp, 10_000);

/** The first 7 characters of a commit. */
export const shortCommit = (commit: string): string => commit.slice(0, 7);

/** The jobs that didn't succeed, by name, in the order the record lists them. */
export function failedJobs(night: Night): readonly string[] {
  return Object.entries(night.jobs)
    .filter(([, result]) => result !== "success")
    .map(([job]) => jobName(job));
}

/** Why `section` has no figure on `night`: its job's result, or that it didn't run. */
export function missing(night: Night, section: keyof typeof SECTION_JOBS): string {
  const job = SECTION_JOBS[section];
  const result = night.jobs[job];
  const what = result === undefined ? HOW_WE_TEST.missing.notRun : (HOW_WE_TEST.missing.results[result] ?? result);
  return `${HOW_WE_TEST.missing.lead} ${jobName(job)} ${what}.`;
}

/**
 * The warning a present `section` carries when its job didn't succeed on some run that night (the
 * figure is from the night's last run), or `undefined` when it succeeded.
 */
export function partial(night: Night, section: keyof typeof SECTION_JOBS): string | undefined {
  const job = SECTION_JOBS[section];
  const result = night.jobs[job];
  if (result === undefined || result === "success") return undefined;
  return HOW_WE_TEST.missing.partial(jobName(job), HOW_WE_TEST.missing.results[result] ?? result);
}
