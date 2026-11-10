// The release gate over the harness (web-app §4, ADR 0018 §2): the median of several runs of each
// gated figure against its §4 target plus a margin for a shared runner's noise. A miss fails the
// gate unless waivers.json names it with its issue. `pnpm --filter @laminapdf/viewer perf:gate --
// <run.json>…` (or `cargo xtask viewer-perf --gate`) prints the table and exits 1 on a miss.

import { readFileSync, writeFileSync } from "node:fs";
import process, { argv } from "node:process";
import { TARGETS } from "./report.ts";

/** How far over its target a median may be before it fails: a hosted runner's noise. */
export const MARGIN = 0.25;

const ns = (ms: number) => ms * 1e6;

/** The gated figures, by their history name, with their §4 targets in the history's units. */
export const GATED: Readonly<Record<string, { readonly name: string; readonly target: number }>> = {
  "first-page-warm": { name: "First page, warm", target: ns(TARGETS.firstPageWarmMs) },
  "first-page-cold": { name: "First page, cold on Fast 4G", target: ns(TARGETS.firstPageColdMs) },
  "to-sharp": { name: "Sharp after scrolling", target: ns(TARGETS.toSharpMs) },
  "find-first-result": { name: "Find: first result", target: ns(TARGETS.findFirstResultMs) },
  "find-full-pass": { name: "Find: every page", target: ns(TARGETS.findFullMs) },
  "memory-peak-bytes": { name: "Memory, 1,000 pages", target: TARGETS.memoryBytes },
};

/** A miss the owner accepted for a release: the figure, the issue and the version that fixes it. */
export interface Waiver {
  readonly figure: string;
  readonly issue: string;
  readonly until: string;
}

/** One gated figure: its median over the runs, the limit and the outcome. */
export interface GateRow {
  readonly figure: string;
  readonly median: number | null;
  readonly limit: number;
  readonly pass: boolean;
  readonly waiver?: Waiver;
}

const median = (values: readonly number[]): number | null => {
  if (values.length === 0) return null;
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 1
    ? (sorted[mid] as number)
    : ((sorted[mid - 1] as number) + (sorted[mid] as number)) / 2;
};

/**
 * Each gated figure's median over `runs` (each run's history figures) against its target plus
 * {@link MARGIN}. A figure no run measured fails: the gate never passes on what it didn't see. A
 * miss with a waiver is listed with it and doesn't fail the gate.
 */
export function gate(
  runs: readonly Readonly<Record<string, number>>[],
  waivers: readonly Waiver[],
): { rows: GateRow[]; pass: boolean } {
  const rows = Object.entries(GATED).map(([figure, { target }]) => {
    const value = median(runs.flatMap((run) => (run[figure] === undefined ? [] : [run[figure] as number])));
    const limit = target * (1 + MARGIN);
    const pass = value !== null && value <= limit;
    const waiver = pass ? undefined : waivers.find((w) => w.figure === figure);
    return { figure, median: value, limit, pass, ...(waiver === undefined ? {} : { waiver }) };
  });
  return { rows, pass: rows.every((row) => row.pass || row.waiver !== undefined) };
}

const show = (figure: string, value: number) =>
  figure.endsWith("-bytes") ? `${Math.round(value / 2 ** 20)} MB` : `${Math.round(value / 1e6)} ms`;

/** The gate as a Markdown table. */
export function gateTable(rows: readonly GateRow[], runs: number): string {
  const lines = [
    `## Release gate: median of ${runs} runs, target + ${MARGIN * 100} %`,
    "",
    "| Figure | Median | Limit | Result |",
    "| --- | --- | --- | --- |",
    ...rows.map((row) => {
      const result = row.pass
        ? "pass"
        : row.waiver === undefined
          ? "**miss**"
          : `waived until ${row.waiver.until} (${row.waiver.issue})`;
      const value = row.median === null ? "not measured" : show(row.figure, row.median);
      return `| ${GATED[row.figure]?.name ?? row.figure} | ${value} | ${show(row.figure, row.limit)} | ${result} |`;
    }),
  ];
  return `${lines.join("\n")}\n`;
}

// Run as a script: the runs' JSON files (run.ts's <browser>.json) and an optional --out file.
if (import.meta.filename === argv[1]) {
  const files = argv.slice(2).filter((arg) => arg !== "--" && arg.endsWith(".json"));
  const runs = files.map(
    (file) => (JSON.parse(readFileSync(file, "utf8")) as { history: Record<string, number> }).history,
  );
  const waivers = JSON.parse(readFileSync(new URL("waivers.json", import.meta.url), "utf8")).waivers as Waiver[];
  const { rows, pass } = gate(runs, waivers);
  const table = gateTable(rows, runs.length);
  console.log(table);
  const out = argv[argv.indexOf("--out") + 1];
  if (argv.includes("--out") && out !== undefined) writeFileSync(out, table);
  if (!pass) {
    console.error("viewer perf: a gated figure missed its §4 target with no waiver");
    process.exitCode = 1;
  }
}
