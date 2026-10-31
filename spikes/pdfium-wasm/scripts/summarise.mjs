// Turns out/results-<browser>.json into the README's Markdown tables (printed to stdout).
//
//   node scripts/summarise.mjs
import { existsSync, readFileSync } from "node:fs";

const browsers = ["chromium", "firefox", "webkit"].filter((b) => existsSync(new URL(`../out/results-${b}.json`, import.meta.url)));
const data = Object.fromEntries(
  browsers.map((b) => [b, JSON.parse(readFileSync(new URL(`../out/results-${b}.json`, import.meta.url), "utf8"))]),
);
const ms = (v) => (typeof v === "number" ? `${v < 10 ? v.toFixed(1) : Math.round(v)}` : "—");
const mb = (v) => (typeof v === "number" ? (v / 1048576).toFixed(0) : "—");
const median = (xs) => {
  const s = xs.filter((x) => typeof x === "number").sort((a, b) => a - b);
  return s.length === 0 ? undefined : s[Math.floor(s.length / 2)];
};

/** Every result of `browser`, with its run's label. */
function results(browser) {
  return data[browser].runs.flatMap((run) => run.results.map((r, i) => ({ ...r, run: run.label, index: i })));
}
function find(browser, file, strategy = "copy", index = 0) {
  return results(browser).filter((r) => r.file === file && (r.strategy ?? "copy") === strategy)[index];
}
function table(head, rows) {
  return [`| ${head.join(" | ")} |`, `| ${head.map((_, i) => (i === 0 ? "---" : "---:")).join(" | ")} |`, ...rows.map((r) => `| ${r.join(" | ")} |`)].join("\n");
}

const out = [];
out.push("**User agents:**", ...browsers.map((b) => `- ${b}: ${data[b].userAgent}`), "");

out.push("### Instantiate (ms; median over the runs' fresh workers)", "");
out.push(
  table(
    ["", ...browsers],
    ["laminaMs", "pdfiumMs", "combinedMs"].map((k) => [k, ...browsers.map((b) => ms(median(data[b].runs.map((r) => r.boot[k]))))]),
  ),
);

out.push("", "### First page visible (ms, from the File handed to the worker to page 1 drawn on the page)", "");
out.push(
  table(
    ["File", ...browsers],
    [
      ["ten-page, cold (first file after boot)", (b) => find(b, "ten-page.pdf")?.firstPageVisibleMs],
      ["ten-page, warm (second time)", (b) => find(b, "ten-page.pdf", "copy", 1)?.firstPageVisibleMs],
      ["ten-page, custom file access", (b) => find(b, "ten-page.pdf", "custom")?.firstPageVisibleMs],
      ["thousand-page, copy", (b) => find(b, "thousand-page.pdf")?.firstPageVisibleMs],
      ["thousand-page, custom", (b) => find(b, "thousand-page.pdf", "custom")?.firstPageVisibleMs],
      ["scan-100mb, copy", (b) => find(b, "scan-100mb.pdf")?.firstPageVisibleMs],
      ["scan-100mb, custom", (b) => find(b, "scan-100mb.pdf", "custom")?.firstPageVisibleMs],
    ].map(([label, f]) => [label, ...browsers.map((b) => ms(f(b)))]),
  ),
);

out.push("", "### Render steps, ten-page page 1 (ms: PDFium render / copy out of the heap / createImageBitmap)", "");
out.push(
  table(
    ["", ...browsers],
    ["first@1x", "first@2x", "middle@1x", "middle@2x"].map((k) => [
      k,
      ...browsers.map((b) => {
        const s = find(b, "ten-page.pdf")?.steps?.[k];
        return s === undefined ? "—" : `${ms(s.renderMs)} / ${ms(s.copyMs)} / ${ms(s.bitmapMs)} (${s.width}×${s.height})`;
      }),
    ]),
  ),
);

out.push("", "### Per-page render, 1× (ms, PDFium render + copy + ImageBitmap; median / p95)", "");
const corpusMedian = (b) => median(results(b).filter((r) => r.file.startsWith("corpus/") && r.renderMs).map((r) => r.renderMs.median));
out.push(
  table(
    ["File", ...browsers],
    [
      ["thousand-page, every page", (b) => find(b, "thousand-page.pdf")?.renderMs],
      ["scan-100mb, every page", (b) => find(b, "scan-100mb.pdf")?.renderMs],
      ["ten-page", (b) => find(b, "ten-page.pdf")?.renderMs],
    ]
      .map(([label, f]) => [label, ...browsers.map((b) => {
        const r = f(b);
        return r === undefined ? "—" : `${ms(r.median)} / ${ms(r.p95)}`;
      })])
      .concat([["corpus sample (median of the files' medians)", ...browsers.map((b) => ms(corpusMedian(b)))]]),
  ),
);

out.push("", "### Peak WebAssembly memory (MiB: lamina base module / PDFium), per input strategy", "");
out.push(
  table(
    ["File, strategy", ...browsers],
    [
      ["thousand-page, copy", "thousand-page.pdf", "copy"],
      ["thousand-page, custom", "thousand-page.pdf", "custom"],
      ["scan-100mb, copy", "scan-100mb.pdf", "copy"],
      ["scan-100mb, custom", "scan-100mb.pdf", "custom"],
      ["reload-50mb, copy", "reload-50mb.pdf", "copy"],
      ["reload-500mb, copy", "reload-500mb.pdf", "copy"],
      ["reload-500mb, custom", "reload-500mb.pdf", "custom"],
    ].map(([label, file, strategy]) => [
      label,
      ...browsers.map((b) => {
        const r = find(b, file, strategy);
        if (r?.error) return "fails";
        const p = r?.peakWasmMemory;
        return p === undefined ? "—" : `${mb(p.base)} / ${mb(p.pdfium)}`;
      }),
    ]),
  ),
);
const ua = browsers.filter((b) => results(b).some((r) => typeof r.uaMemoryBytes === "number"));
if (ua.length > 0) {
  out.push(
    "",
    `measureUserAgentSpecificMemory (${ua.join(", ")}; MiB, after the run): ` +
      ["thousand-page.pdf", "scan-100mb.pdf"]
        .flatMap((f) => ["copy", "custom"].map((s) => `${f} ${s} ${mb(find(ua[0], f, s)?.uaMemoryBytes)}`))
        .join(", "),
  );
}

out.push("", "### Reload after an incremental save (ms: lamina edit + save / PDFium close / copy + reopen / re-render page 1 / total)", "");
out.push(
  table(
    ["Input", ...browsers],
    [
      ["ten-page (1 MB)", "ten-page.pdf"],
      ["reload-50mb", "reload-50mb.pdf"],
      ["reload-50mb, heap already grown by the 100 MB file", "reload-50mb.pdf", 1],
      ["reload-50mb, after a small save in the same worker", "reload-50mb.pdf", 2],
      ["reload-500mb", "reload-500mb.pdf"],
      ["reload-50mb, custom (appended revision only, reopened by ranges)", "reload-50mb.pdf", 0, "custom"],
      ["reload-500mb, custom", "reload-500mb.pdf", 0, "custom"],
    ].map(([label, file, index = 0, strategy = "copy"]) => [
      label,
      ...browsers.map((b) => {
        const r = find(b, file, strategy, index);
        if (r?.error) return `fails: ${r.error.split("\n")[0].replace(/^Error: page.evaluate: /, "")}`;
        const x = r?.reload;
        return x === undefined ? "—" : `${ms(x.saveMs)} / ${ms(x.closeMs)} / ${ms(x.copyAndReopenMs)} / ${ms(x.rerenderMs)} / **${ms(x.totalMs)}**`;
      }),
    ]),
  ),
);

out.push("", "### Peak WebAssembly memory during the reload (MiB: lamina / PDFium, from the save to the re-render)", "");
out.push(
  table(
    ["Input", ...browsers],
    [
      ["reload-50mb, copy", "reload-50mb.pdf", "copy"],
      ["reload-50mb, custom", "reload-50mb.pdf", "custom"],
      ["reload-500mb, copy", "reload-500mb.pdf", "copy"],
      ["reload-500mb, custom", "reload-500mb.pdf", "custom"],
    ].map(([label, file, strategy]) => [
      label,
      ...browsers.map((b) => {
        const r = find(b, file, strategy);
        if (r?.error) return "fails";
        const p = r?.reload?.peakWasmMemory;
        return p === undefined ? "—" : `${mb(p.base)} / ${mb(p.pdfium)} (appended ${(r.reload.appendedBytes / 1024).toFixed(1)} KiB)`;
      }),
    ]),
  ),
);

out.push("", "### Encrypted files (both engines, password inside the worker only)", "");
out.push(
  table(
    ["File", ...browsers],
    [
      ["r6 AES-256 (`Pässwört-6`), copy", "encrypted-r6.pdf", "copy"],
      ["r6 AES-256, custom", "encrypted-r6.pdf", "custom"],
      ["r4 AES-128 (`user-pw`), copy", "encrypted-r4.pdf", "copy"],
    ].map(([label, file, strategy]) => [
      label,
      ...browsers.map((b) => {
        const r = find(b, file, strategy);
        return r?.error ? `fails: ${r.error.split("\n")[0]}` : r ? `opens, ${r.pages} page(s) (PDFium ${r.pdfiumPages})` : "—";
      }),
    ]),
  ),
);
console.log(out.join("\n"));
