// src/data/benchmarks.json, read for the benchmarks page: `cargo xtask benchmarks` writes it from
// the measuring tasks' reports, so no number on the page is typed by hand. `parseBenchmarks`
// checks the file's shape and throws on a field it doesn't describe, which fails the build;
// `sectionParts` gives a section's table, summary and run, which the page (Benchmarks.astro) draws
// and `sectionMarkdown` writes for its Markdown twin.
// Pure functions with no imports, so Node runs this file as it is for the twin
// (plugins/agent-docs.mjs) and its test (plugins/benchmarks.test.mjs).

/** The machine a run used; a value the machine didn't give is `null`. */
export interface Machine {
  readonly os: string | null;
  readonly cpu: string | null;
  readonly cores: number | null;
  readonly memoryBytes: number | null;
  readonly loadAverage: number | null;
}

/** What a section says about its run. */
export interface Run {
  readonly version: string;
  readonly commit: string;
  /** `YYYY-MM-DD`, UTC. */
  readonly date: string;
  readonly machine: Machine;
}

/** One runtime's timings in milliseconds and its WebAssembly memory in bytes. */
export interface RuntimeRow {
  readonly runtime: string;
  /** The version of the tool the runtime runs on (Node, Deno, Bun, Playwright or wrangler). */
  readonly version: string | null;
  readonly cases: Readonly<Record<string, number>>;
}

/** The generation section: per runtime the timings, and the native Rust row. */
export interface Generation {
  readonly run: Run;
  readonly runtimes: readonly RuntimeRow[];
  readonly rust: { readonly toolchain: string; readonly cases: Readonly<Record<string, number>> };
}

/** One WebAssembly module: bytes raw, gzipped and brotli-compressed, and its gzip budget. */
export interface ModuleSize {
  readonly name: string;
  readonly raw: number;
  readonly gzip: number;
  readonly brotli: number;
  readonly budgetGzip: number | null;
}

/** One JavaScript bundle: bytes minified and gzipped, and its gzip budget. */
export interface BundleSize {
  readonly name: string;
  readonly minified: number;
  readonly gzip: number;
  readonly budgetGzip: number | null;
}

/** The sizes section: the WebAssembly modules and the JavaScript bundles. */
export interface Sizes {
  readonly run: Run;
  readonly modules: readonly ModuleSize[];
  readonly js: readonly BundleSize[];
}

/** One benchmark subset: its pages, tests, passes and score. */
export interface SubsetScore {
  readonly name: string;
  readonly measures: string;
  readonly pdfs: number;
  readonly tests: number;
  readonly passed: number;
  /** 0 to 1. */
  readonly score: number;
}

/** One extraction run on the benchmark: its dataset, scores, pages and speed. */
export interface Extraction {
  readonly dataset: {
    readonly name: string;
    readonly title: string;
    readonly url: string;
    readonly revision: string;
    readonly licence: string;
    readonly attribution: string;
  };
  readonly candidate: string;
  readonly tables: string;
  readonly run: Run;
  readonly subsets: readonly SubsetScore[];
  readonly baseline: { readonly tests: number; readonly passed: number; readonly score: number };
  readonly overall: number;
  readonly overallWithoutBaseline: number;
  readonly pages: {
    readonly extracted: number;
    readonly failed: number;
    readonly panicked: number;
    readonly crashedOrTimedOut: number;
  };
  readonly speed: {
    readonly threads: number;
    readonly totalMillis: number;
    readonly medianMillisPerPage: number;
    readonly pagesPerSecond: number;
  };
}

/** The whole data file, checked by {@link parseBenchmarks}. */
export interface Benchmarks {
  readonly version: 1;
  readonly generation: Generation;
  readonly sizes: Sizes;
  /** By candidate: `laminapdf` (the default rules), `laminapdf-stream` (also unruled tables). */
  readonly extraction: Readonly<Record<string, Extraction>>;
}

/** A table: its heading cells and rows of cell text (Markdown inline syntax). */
export interface Table {
  readonly headers: readonly string[];
  /** Whether each column holds numbers (right-aligned). */
  readonly numeric: readonly boolean[];
  readonly rows: readonly (readonly string[])[];
}

/** A reader of one value at `path`, for error messages. */
class Field {
  private readonly value: unknown;
  private readonly path: string;

  constructor(value: unknown, path: string) {
    this.value = value;
    this.path = path;
  }

  private fail(what: string): never {
    throw new Error(`benchmarks.json: ${this.path} should be ${what}`);
  }

  get(key: string): Field {
    const v = this.value;
    if (typeof v !== "object" || v === null || Array.isArray(v)) this.fail("an object");
    return new Field((v as Record<string, unknown>)[key], `${this.path}.${key}`);
  }

  string(): string {
    return typeof this.value === "string" ? this.value : this.fail("a string");
  }

  number(): number {
    return typeof this.value === "number" && Number.isFinite(this.value) ? this.value : this.fail("a number");
  }

  stringOrNull(): string | null {
    return this.value === null ? null : this.string();
  }

  numberOrNull(): number | null {
    return this.value === null ? null : this.number();
  }

  array(): Field[] {
    if (!Array.isArray(this.value)) this.fail("an array");
    return (this.value as unknown[]).map((v, i) => new Field(v, `${this.path}[${i}]`));
  }

  entries(): [string, Field][] {
    const v = this.value;
    if (typeof v !== "object" || v === null || Array.isArray(v)) this.fail("an object");
    return Object.keys(v as object).map((k) => [k, this.get(k)]);
  }
}

function run(f: Field): Run {
  const m = f.get("machine");
  return {
    version: f.get("version").string(),
    commit: f.get("commit").string(),
    date: f.get("date").string(),
    machine: {
      os: m.get("os").stringOrNull(),
      cpu: m.get("cpu").stringOrNull(),
      cores: m.get("cores").numberOrNull(),
      memoryBytes: m.get("memoryBytes").numberOrNull(),
      loadAverage: m.get("loadAverage").numberOrNull(),
    },
  };
}

function numbers(f: Field): Record<string, number> {
  return Object.fromEntries(f.entries().map(([k, v]) => [k, v.number()]));
}

function extraction(f: Field): Extraction {
  const d = f.get("dataset");
  const b = f.get("baseline");
  const p = f.get("pages");
  const s = f.get("speed");
  return {
    dataset: {
      name: d.get("name").string(),
      title: d.get("title").string(),
      url: d.get("url").string(),
      revision: d.get("revision").string(),
      licence: d.get("licence").string(),
      attribution: d.get("attribution").string(),
    },
    candidate: f.get("candidate").string(),
    tables: f.get("tables").string(),
    run: run(f.get("run")),
    subsets: f
      .get("subsets")
      .array()
      .map((x) => ({
        name: x.get("name").string(),
        measures: x.get("measures").string(),
        pdfs: x.get("pdfs").number(),
        tests: x.get("tests").number(),
        passed: x.get("passed").number(),
        score: x.get("score").number(),
      })),
    baseline: { tests: b.get("tests").number(), passed: b.get("passed").number(), score: b.get("score").number() },
    overall: f.get("overall").number(),
    overallWithoutBaseline: f.get("overallWithoutBaseline").number(),
    pages: {
      extracted: p.get("extracted").number(),
      failed: p.get("failed").number(),
      panicked: p.get("panicked").number(),
      crashedOrTimedOut: p.get("crashedOrTimedOut").number(),
    },
    speed: {
      threads: s.get("threads").number(),
      totalMillis: s.get("totalMillis").number(),
      medianMillisPerPage: s.get("medianMillisPerPage").number(),
      pagesPerSecond: s.get("pagesPerSecond").number(),
    },
  };
}

/** The data file, checked; throws naming the first field that is missing or of the wrong type. */
export function parseBenchmarks(raw: unknown): Benchmarks {
  const root = new Field(raw, "benchmarks");
  if (root.get("version").number() !== 1) throw new Error("benchmarks.json: version should be 1");
  const g = root.get("generation");
  const sz = root.get("sizes");
  return {
    version: 1,
    generation: {
      run: run(g.get("run")),
      runtimes: g
        .get("runtimes")
        .array()
        .map((r) => ({
          runtime: r.get("runtime").string(),
          version: r.get("version").stringOrNull(),
          cases: numbers(r.get("cases")),
        })),
      rust: { toolchain: g.get("rust").get("toolchain").string(), cases: numbers(g.get("rust").get("cases")) },
    },
    sizes: {
      run: run(sz.get("run")),
      modules: sz
        .get("modules")
        .array()
        .map((m) => ({
          name: m.get("name").string(),
          raw: m.get("raw").number(),
          gzip: m.get("gzip").number(),
          brotli: m.get("brotli").number(),
          budgetGzip: m.get("budgetGzip").numberOrNull(),
        })),
      js: sz
        .get("js")
        .array()
        .map((j) => ({
          name: j.get("name").string(),
          minified: j.get("minified").number(),
          gzip: j.get("gzip").number(),
          budgetGzip: j.get("budgetGzip").numberOrNull(),
        })),
    },
    extraction: Object.fromEntries(
      root
        .get("extraction")
        .entries()
        .map(([k, v]) => [k, extraction(v)]),
    ),
  };
}

const MONTHS = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
];

/** `2026-11-08` → `8 November 2026`. */
function longDate(date: string): string {
  const [y, m, d] = date.split("-").map(Number);
  const month = MONTHS[(m ?? 1) - 1];
  return month === undefined || y === undefined || d === undefined ? date : `${d} ${month} ${y}`;
}

/** One sentence naming a run: version, commit, date and machine. */
export function runSentence(r: Run): string {
  const m = r.machine;
  const parts = [
    m.cores === null ? null : `${m.cores} cores`,
    m.memoryBytes === null ? null : `${Math.round(m.memoryBytes / 2 ** 30)} GB of memory`,
  ].filter((p): p is string => p !== null);
  const machine = [
    m.cpu ?? "an unnamed processor",
    parts.length > 0 ? ` (${parts.join(", ")})` : "",
    m.os === null ? "" : `, ${m.os}`,
  ].join("");
  const load = m.loadAverage === null ? "" : `, with a load average of ${m.loadAverage.toFixed(1)} when the run began`;
  return `Measured with Lamina PDF ${r.version} (commit \`${r.commit.slice(0, 7)}\`) on ${longDate(r.date)}, on ${machine}${load}.`;
}

/** The runtimes in the order the page lists them; any other label comes last, alphabetically. */
const RUNTIME_ORDER = ["node", "deno", "bun", "workerd", "chromium", "firefox", "webkit"];
const RUNTIME_NAMES: Record<string, string> = {
  deno: "Deno",
  bun: "Bun",
  workerd: "Cloudflare Workers (workerd)",
  chromium: "Chromium",
  firefox: "Firefox",
  webkit: "WebKit (Safari's engine)",
};

function runtimeName(runtime: string): string {
  const node = /^node(?:-(\d+))?$/.exec(runtime);
  if (node) return node[1] === undefined ? "Node" : `Node ${node[1]}`;
  return RUNTIME_NAMES[runtime] ?? runtime;
}

function runtimeRank(runtime: string): number {
  const base = runtime.startsWith("node") ? "node" : runtime;
  const i = RUNTIME_ORDER.indexOf(base);
  return i === -1 ? RUNTIME_ORDER.length : i;
}

/** Milliseconds as shown: one decimal below 100 ms, whole numbers above. */
function ms(value: number | undefined): string {
  if (value === undefined) return "–";
  return value < 100 ? `${value.toFixed(1)} ms` : `${Math.round(value).toLocaleString("en-US")} ms`;
}

/** Bytes as kilobytes (1,000 bytes), as the size budgets count them. */
function kb(bytes: number | null | undefined): string {
  return bytes === null || bytes === undefined
    ? "–"
    : `${(bytes / 1000).toLocaleString("en-US", { maximumFractionDigits: 1, minimumFractionDigits: 1 })} kB`;
}

/** Bytes as megabytes of 1,000,000 bytes, the page's unit as for the sizes. */
function mb(bytes: number | undefined): string {
  return bytes === undefined ? "–" : `${(bytes / 1e6).toFixed(1)} MB`;
}

function percent(share: number): string {
  return `${(share * 100).toFixed(1)} %`;
}

function count(n: number): string {
  return n.toLocaleString("en-US");
}

/** The generation table: one row per runtime, then the native Rust row. */
export function generationTable(g: Generation): Table {
  const rows = [...g.runtimes]
    .sort((a, b) => runtimeRank(a.runtime) - runtimeRank(b.runtime) || a.runtime.localeCompare(b.runtime))
    .map((r) => [
      runtimeName(r.runtime),
      r.version ?? "–",
      ms(r.cases["hello-first"]),
      ms(r.cases.hello),
      ms(r.cases.invoice),
      ms(r.cases.report),
      mb(r.cases["wasm-memory"]),
    ]);
  rows.push(["Rust, native", g.rust.toolchain, "–", "–", ms(g.rust.cases.invoice), ms(g.rust.cases.report), "–"]);
  return {
    headers: ["Runtime", "Version", "First PDF", "Hello", "One-page invoice", "100-page report", "WebAssembly memory"],
    numeric: [false, false, true, true, true, true, true],
    rows,
  };
}

/** The size table: the WebAssembly modules, then the JavaScript bundles. */
export function sizesTable(s: Sizes): Table {
  const modules = s.modules.map((m) => [`\`lamina_pdf_${m.name}.wasm\``, kb(m.gzip), kb(m.brotli), kb(m.budgetGzip)]);
  const js = s.js.map((j) => [`\`${j.name}\` (JavaScript)`, kb(j.gzip), "–", kb(j.budgetGzip)]);
  return {
    headers: ["File", "Gzip", "Brotli", "Budget (gzip)"],
    numeric: [false, true, true, true],
    rows: [...modules, ...js],
  };
}

/** The extraction table: each subset's score, then the benchmark's baseline group. */
export function extractionTable(e: Extraction): Table {
  const rows = e.subsets.map((s) => [
    `\`${s.name}\``,
    s.measures,
    count(s.pdfs),
    count(s.tests),
    count(s.passed),
    percent(s.score),
  ]);
  rows.push([
    "Baseline",
    "the page gave text, without long repeats (one test per page)",
    "–",
    count(e.baseline.tests),
    count(e.baseline.passed),
    percent(e.baseline.score),
  ]);
  return {
    headers: ["Subset", "What it checks", "Pages", "Tests", "Passed", "Score"],
    numeric: [false, false, true, true, true, true],
    rows,
  };
}

/** The extraction run's speed and overall score, in sentences. */
export function extractionSummary(e: Extraction): string {
  const pps = e.speed.pagesPerSecond;
  return (
    `Overall, as the benchmark averages its groups, ${percent(e.overall)}; without the baseline group, ${percent(e.overallWithoutBaseline)}. ` +
    `${count(e.pages.extracted)} pages were extracted and ${count(e.pages.failed + e.pages.panicked + e.pages.crashedOrTimedOut)} failed. ` +
    `On one thread that is ${pps >= 100 ? count(Math.round(pps)) : pps.toFixed(1)} pages a second, with a median of ${e.speed.medianMillisPerPage.toFixed(1)} ms a page.`
  );
}

function escapeCell(text: string): string {
  return text.replaceAll("|", "\\|").replaceAll("\n", " ");
}

/** `t` as a Markdown pipe table. */
export function markdownTable(t: Table): string {
  const line = (cells: readonly string[]) => `| ${cells.map(escapeCell).join(" | ")} |`;
  return [line(t.headers), line(t.numeric.map((n) => (n ? "---:" : "---"))), ...t.rows.map(line)].join("\n");
}

/** The sections the page places, by the name its `<Benchmarks section>` gives. */
export type Section = "generation" | "sizes" | "extraction" | "extraction-stream";

/** What a section shows: its table, a summary for extraction, and its run. */
export interface SectionParts {
  readonly table: Table;
  readonly summary?: string;
  readonly run: Run;
}

/** `section`'s parts, or `undefined` for an extraction candidate the file doesn't hold. */
export function sectionParts(b: Benchmarks, section: Section): SectionParts | undefined {
  switch (section) {
    case "generation":
      return { table: generationTable(b.generation), run: b.generation.run };
    case "sizes":
      return { table: sizesTable(b.sizes), run: b.sizes.run };
    case "extraction":
    case "extraction-stream": {
      const e = b.extraction[section === "extraction" ? "laminapdf" : "laminapdf-stream"];
      return e === undefined ? undefined : { table: extractionTable(e), summary: extractionSummary(e), run: e.run };
    }
  }
}

/** A section as Markdown: its table, its summary for extraction, and its run sentence. */
export function sectionMarkdown(b: Benchmarks, section: Section): string {
  const parts = sectionParts(b, section);
  if (parts === undefined) return "Not measured yet.";
  return [markdownTable(parts.table), parts.summary, runSentence(parts.run)]
    .filter((p) => p !== undefined)
    .join("\n\n");
}
