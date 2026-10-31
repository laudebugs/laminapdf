// The benchmark (by hand, not in CI): every measured run gets a fresh page, so a fresh worker and
// fresh WebAssembly memories (memory never shrinks, so peaks would add up across files). The
// 10-page file runs twice on one page: cold (right after boot) and warm. Results go to
// out/results-<browser>.json. PDFIUM_SPIKE_LARGE=0 skips the 500 MB reload file.
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { expect, test } from "@playwright/test";

const fixtures = new URL("../out/public/fixtures/", import.meta.url);
const corpus = JSON.parse(readFileSync(new URL("index.json", fixtures), "utf8"));
const large = process.env.PDFIUM_SPIKE_LARGE !== "0";
const f = (name, extra = {}) => ({ url: `/fixtures/${name}`, name, ...extra });

/** Each entry is one fresh page: its jobs run there in order. */
const runs = [
  { label: "ten-page cold then warm", jobs: [f("ten-page.pdf", { reload: true }), f("ten-page.pdf")] },
  { label: "ten-page custom", jobs: [f("ten-page.pdf", { strategy: "custom" })] },
  { label: "thousand copy", jobs: [f("thousand-page.pdf", { allPages: true })] },
  { label: "thousand custom", jobs: [f("thousand-page.pdf", { allPages: true, strategy: "custom" })] },
  { label: "scan copy", jobs: [f("scan-100mb.pdf")] },
  { label: "scan custom", jobs: [f("scan-100mb.pdf", { strategy: "custom" })] },
  { label: "reload 50 MB", jobs: [f("reload-50mb.pdf", { reload: true })] },
  // The same save with the heap already grown by a larger file: is a fresh heap's growth the cost?
  { label: "reload 50 MB, grown heap", jobs: [f("scan-100mb.pdf"), f("reload-50mb.pdf", { reload: true })] },
  // After a small save in the same worker: does Chromium's tiering (functions start in the baseline
  // compiler, optimised only after many calls) explain a slow first large save?
  { label: "reload 50 MB after a small save", jobs: [f("ten-page.pdf", { reload: true }), f("reload-50mb.pdf", { reload: true })] },
  ...(large ? [{ label: "reload 500 MB", jobs: [f("reload-500mb.pdf", { reload: true })] }] : []),
  {
    label: "encrypted",
    jobs: [
      // No reload: lamina refuses incremental saves on an encrypted base (encrypted-base, until E11).
      f("encrypted-r6.pdf", { password: "Pässwört-6" }),
      f("encrypted-r6.pdf", { password: "Pässwört-6", strategy: "custom" }),
      f("encrypted-r4.pdf", { password: "user-pw" }),
    ],
  },
  { label: "corpus", jobs: corpus.map((c) => f(`corpus/${c.name}`)) },
];

test("both engines in one worker: boot, render, reload, input strategies", async ({ browser, browserName }) => {
  const out = [];
  let page;
  for (const run of runs) {
    // A fresh context per run: an empty cache too (WebKit refuses a cached worker script whose
    // 304 lacks the cross-origin isolation headers).
    await page?.context().close();
    page = await (await browser.newContext({ baseURL: "http://localhost:5199" })).newPage();
    page.on("pageerror", (e) => console.log(`[${browserName}] page error: ${e.message}`));
    await page.goto("/");
    const boot = await page.evaluate(() => window.spike.boot());
    const results = [];
    for (const job of run.jobs) {
      try {
        results.push(await page.evaluate((j) => window.spike.run([j]).then((r) => r[0]), job));
      } catch (e) {
        results.push({ file: job.name, strategy: job.strategy ?? "copy", error: String(e).slice(0, 400) });
      }
    }
    out.push({ label: run.label, boot, results });
    console.log(`[${browserName}] ${run.label}: ${results.map((r) => r.error ?? `${Math.round(r.firstPageVisibleMs)} ms`).join(", ")}`);
  }
  mkdirSync(new URL("../out/", import.meta.url), { recursive: true });
  writeFileSync(
    new URL(`../out/results-${browserName}.json`, import.meta.url),
    `${JSON.stringify({ browser: browserName, userAgent: await page.evaluate(() => navigator.userAgent), runs: out }, null, 1)}\n`,
  );
  // Page 1 of the three reference files rendered in this browser, by both strategies.
  const all = out.flatMap((r) => r.results);
  for (const name of ["ten-page.pdf", "thousand-page.pdf", "scan-100mb.pdf"]) {
    for (const strategy of ["copy", "custom"]) {
      const r = all.find((x) => x.file === name && x.strategy === strategy);
      expect(r?.error, `${name} (${strategy}): ${r?.error}`).toBeUndefined();
      expect(r?.steps?.["first@1x"]?.width, `${name} (${strategy})`).toBeGreaterThan(0);
    }
  }
});
