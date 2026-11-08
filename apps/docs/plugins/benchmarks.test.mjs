// src/lib/benchmarks.ts: the reader refuses a file it doesn't describe, accepts the committed one,
// and its tables show each runtime's invoice and report timings, each module's gzip size and each
// subset's score.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import {
  extractionSummary,
  extractionTable,
  generationTable,
  markdownTable,
  parseBenchmarks,
  runSentence,
  sectionMarkdown,
  sizesTable,
} from "../src/lib/benchmarks.ts";

const run = {
  version: "0.2.0-alpha.3",
  commit: "0123456789abcdef",
  date: "2026-11-08",
  machine: { os: "macOS 27.0.1", cpu: "Apple M1 Pro", cores: 8, memoryBytes: 17179869184, loadAverage: 2.04 },
};

/** A small file of the committed shape. */
function sample() {
  return {
    version: 1,
    generation: {
      run,
      runtimes: [
        {
          runtime: "chromium",
          version: "1.64.1",
          cases: { "hello-first": 3.2, hello: 0.4, invoice: 2.25, report: 160.4, "wasm-memory": 4200000 },
        },
        { runtime: "node-24", version: "24.19.0", cases: { hello: 0.3, invoice: 1.9, report: 140 } },
      ],
      rust: { toolchain: "1.99.1", cases: { invoice: 0.8, report: 45.25 } },
    },
    sizes: {
      run,
      modules: [{ name: "base", raw: 1111073, gzip: 399531, brotli: 318199, budgetGzip: 410000 }],
      js: [{ name: "@laminapdf/sdk", minified: 88728, gzip: 18341, budgetGzip: 20000 }],
    },
    extraction: {
      laminapdf: {
        dataset: {
          name: "olmocr-bench",
          title: "olmOCR-bench",
          url: "https://huggingface.co/datasets/allenai/olmOCR-bench",
          revision: "54a96a6fb6a2",
          licence: "ODC-BY-1.0",
          attribution: "olmOCR-bench, Allen Institute for AI",
        },
        candidate: "laminapdf",
        tables: "lattice",
        sample: null,
        run,
        subsets: [{ name: "multi_column", measures: "one passage before another", pdfs: 231, tests: 884, passed: 461, score: 0.5216 }],
        baseline: { tests: 231, passed: 191, score: 0.8268 },
        overall: 0.674,
        overallWithoutBaseline: 0.5215,
        pages: { extracted: 228, failed: 3, panicked: 0, crashedOrTimedOut: 0 },
        speed: { threads: 1, totalMillis: 600, medianMillisPerPage: 2.1, pagesPerSecond: 380 },
      },
    },
  };
}

test("the reader accepts a file of the committed shape", () => {
  const b = parseBenchmarks(sample());
  assert.equal(b.generation.runtimes.length, 2);
  assert.equal(b.extraction.laminapdf?.speed.pagesPerSecond, 380);
});

test("the reader accepts the committed data file", async () => {
  const text = await readFile(new URL("../src/data/benchmarks.json", import.meta.url), "utf8");
  assert.doesNotThrow(() => parseBenchmarks(JSON.parse(text)));
});

test("the reader names a missing field", () => {
  const broken = sample();
  delete broken.sizes.modules[0].gzip;
  assert.throws(() => parseBenchmarks(broken), /benchmarks\.sizes\.modules\[0\]\.gzip should be a number/);
});

test("the reader refuses another version of the file", () => {
  assert.throws(() => parseBenchmarks({ ...sample(), version: 2 }), /version should be 1/);
});

test("the generation table shows each runtime's invoice and report, then native Rust", () => {
  const t = generationTable(parseBenchmarks(sample()).generation);
  assert.deepEqual(
    t.rows.map((r) => [r[0], r[4], r[5]]),
    [
      ["Node 24", "1.9 ms", "140 ms"],
      ["Chromium", "2.3 ms", "160 ms"],
      ["Rust, native", "0.8 ms", "45.3 ms"],
    ],
  );
});

test("a case a runtime didn't measure is a dash", () => {
  const t = generationTable(parseBenchmarks(sample()).generation);
  assert.equal(t.rows[0]?.[2], "–");
});

test("the size table shows each module's gzip size against its budget", () => {
  const t = sizesTable(parseBenchmarks(sample()).sizes);
  assert.deepEqual(t.rows[0], ["`lamina_pdf_base.wasm`", "399.5 kB", "318.2 kB", "410.0 kB"]);
});

test("the extraction table shows each subset's score and the baseline", () => {
  const e = parseBenchmarks(sample()).extraction.laminapdf;
  assert.ok(e);
  const t = extractionTable(e);
  assert.deepEqual(t.rows[0]?.slice(2), ["231", "884", "461", "52.2 %"]);
  assert.equal(t.rows[1]?.[0], "Baseline");
});

test("the extraction summary gives the pages a second and the median page", () => {
  const e = parseBenchmarks(sample()).extraction.laminapdf;
  assert.ok(e);
  assert.match(extractionSummary(e), /380 pages a second, with a median of 2\.1 ms a page/);
});

test("a run sentence names the version, short commit, date and machine", () => {
  assert.equal(
    runSentence(parseBenchmarks(sample()).generation.run),
    "Measured with Lamina PDF 0.2.0-alpha.3 (commit `0123456`) on 8 November 2026, on Apple M1 Pro (8 cores, 16 GB of memory), macOS 27.0.1, with a load average of 2.0 when the run began.",
  );
});

test("a Markdown table right-aligns numbers and escapes pipes", () => {
  const md = markdownTable({ headers: ["A", "B"], numeric: [false, true], rows: [["x|y", "1"]] });
  assert.equal(md, "| A | B |\n| --- | ---: |\n| x\\|y | 1 |");
});

test("a section that wasn't measured says so", () => {
  assert.equal(sectionMarkdown(parseBenchmarks(sample()), "extraction-stream"), "Not measured yet.");
});
