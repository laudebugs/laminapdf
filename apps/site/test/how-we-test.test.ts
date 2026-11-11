// The How we test page rendered from fixture snapshots: every figure, the night's date and its short
// commit come from proof.json; a section whose job left no report says so and shows no zero; the
// licences are deny.toml's, every one.

import { experimental_AstroContainer as AstroContainer } from "astro/container";
import { describe, expect, it } from "vitest";
import HowWeTest from "../src/components/how-we-test/HowWeTest.astro";
import { percent } from "../src/data/how-we-test.js";
import type { Night, Proof } from "../src/data/proof.js";
import { licencePolicy } from "../src/data/sources.js";

const SUCCESS = {
  differential: "success",
  fuzz: "success",
  "render-diff": "success",
  verapdf: "success",
  roundtrip: "success",
  "pdf-runtimes": "success",
};

const NIGHT: Night = {
  commit: "abcdef0123456789abcdef0123456789abcdef01",
  ran: "2031-02-03",
  lamina: "9.8.7-alpha.6",
  passed: true,
  jobs: SUCCESS,
  corpus: {
    complete: true,
    sources: {
      qpdf: { files: 4321, opened: 4000, repaired: 300, refused: 21, refusedAlone: 7, errors: 0 },
      mutants: { files: 1234, opened: 234, repaired: 900, refused: 100, refusedAlone: 33, errors: 0 },
    },
  },
  differential: {
    inputs: 5555,
    failures: 0,
    tools: {
      pdfjs: {
        version: "pdfjs-dist 1.2.3",
        facts: {
          outcome: { agree: 5000, disagree: 555, excused: 555 },
          pageCount: { agree: 4100, disagree: 200, excused: 200 },
          outlineTitles: { agree: 4299, disagree: 1, excused: 1 },
        },
      },
    },
  },
  text: { tools: { pdfjs: "1.2.3" }, sources: { qpdf: { pages: 900, compared: 880, agree: 777 } } },
  fuzz: { targets: 13, seconds: 600, executions: 98_765_432, crashed: 0 },
  verapdf: {
    version: "1.30.3",
    validations: [{ file: "fixture-report.pdf", profile: "ua1", compliant: true }],
  },
  roundtrip: { cases: 640, saved: 600, refused: 40, tools: { qpdf: true, pdfium: true } },
  runtimes: { passed: ["node", "workerd"], failed: [], missing: [] },
  render: { dpi: 96, pagesPerFile: 20, sources: { all: { pages: 4444, passMinorBp: 9123 } } },
};

const PROOF: Proof = {
  version: 1,
  date: "2031-02-04",
  night: NIGHT,
  nights: { recorded: 30, passed: 27, since: "2031-01-05" },
  fuzzHours: 321,
};

async function render(proof: Proof, pages: readonly string[] = []): Promise<string> {
  const container = await AstroContainer.create();
  return container.renderToString(HowWeTest, { props: { proof, policy: licencePolicy(), pages } });
}

/** `html` as text, tags removed and spaces collapsed. */
function text(html: string): string {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&#39;/g, "'")
    .replace(/&amp;/g, "&")
    .replace(/\s+/g, " ");
}

/** The text of the section whose heading has `id`. */
function section(html: string, id: string): string {
  const start = html.indexOf(`id="${id}"`);
  const end = html.indexOf("<section", start);
  return text(html.slice(start, end < 0 ? undefined : end));
}

describe("the How we test page", () => {
  it("shows the night's date, version, short commit and how many nights passed", async () => {
    const html = await render(PROOF);
    const page = text(html);
    expect(page).toContain("ran on 3 February 2031 and tested version 9.8.7-alpha.6, commit abcdef0 .");
    expect(html).toContain('datetime="2031-02-03"');
    expect(page).toContain("Every check passed that night.");
    expect(page).toContain("27 of the last 30 nights passed every check, counting from 5 January 2031");
  });

  it("shows every figure of the snapshot", async () => {
    const page = text(await render(PROOF));
    for (const figure of [
      // The files: qpdf's row and the mutants'.
      "qpdf 4,321 92.6% 6.9% 0.5% 7",
      "Lamina PDF's own files, damaged on purpose 1,234 19.0% 72.9% 8.1% 33",
      "No file made Lamina PDF crash, hang or panic.",
      // Agreement: pdf.js on the three facts, then page text.
      `pdf.js pdfjs-dist 1.2.3 ${percent(5000, 5555)} ${percent(4100, 4300)} ${percent(4299, 4300)}`,
      "No difference was left unexplained.",
      `qpdf 880 ${percent(777, 880)}`,
      // Fuzzing, saving, the runtimes, veraPDF and the render comparison.
      "13 targets ran for 10 minutes each and tried 98,765,432 inputs. None found a crash.",
      "the fuzzers have run for at least 321 hours in all",
      "Of 640 cases, 600 were saved. Lamina PDF refused the other 40",
      "PDFium read every saved file without an error.",
      "Matched byte for byte: Node.js, Cloudflare Workers.",
      "fixture-report.pdf PDF/UA-1 Passes",
      "91.2% of 4,444 pages look the same or nearly the same.",
    ]) {
      expect(page).toContain(figure);
    }
  });

  it("names the failed checks and says a missing figure is missing, with no zero in its place", async () => {
    const night: Night = { ...NIGHT, passed: false, jobs: { ...SUCCESS, "render-diff": "failure" }, render: null };
    const html = await render({ ...PROOF, night });
    expect(text(html)).toContain("These checks failed that night: the drawing comparison.");
    const rendering = section(html, "rendering");
    expect(rendering).toContain("There is no figure for this night: the drawing comparison failed.");
    expect(rendering).not.toMatch(/\b0(\.0)?%|\b0 pages/);
    expect(rendering).not.toContain("<table");
  });

  it("says a section's job didn't run when the record doesn't name it", async () => {
    const { verapdf: _, ...jobs } = SUCCESS;
    const night: Night = { ...NIGHT, jobs, verapdf: null };
    const standards = section(await render({ ...PROOF, night }), "standards");
    expect(standards).toContain("There is no figure for this night: the PDF/A and PDF/UA checks didn't run.");
  });

  it("warns that a figure is the last run's when its job failed on a run that night", async () => {
    const night: Night = { ...NIGHT, passed: false, jobs: { ...SUCCESS, "render-diff": "failure" } };
    const rendering = section(await render({ ...PROOF, night }), "rendering");
    expect(rendering).toContain("91.2%");
    expect(rendering).toContain(
      "The drawing comparison failed in a run that night; this figure is from the night's last run.",
    );
  });

  it("words the warning by what happened to the job, in every section", async () => {
    const night: Night = { ...NIGHT, passed: false, jobs: { ...SUCCESS, fuzz: "cancelled", verapdf: "failure" } };
    const html = await render({ ...PROOF, night });
    expect(section(html, "fuzzing")).toContain("Fuzzing was cancelled in a run that night");
    expect(section(html, "standards")).toContain("The PDF/A and PDF/UA checks failed in a run that night");
    expect(section(html, "rendering")).not.toContain("in a run that night");
  });

  it("says a night without job results doesn't count as passed, naming no check", async () => {
    const page = text(await render({ ...PROOF, night: { ...NIGHT, passed: false, jobs: {} } }));
    expect(page).toContain("That night's record doesn't say which checks passed");
    expect(page).not.toContain("These checks failed that night");
  });

  it("counts the files that made Lamina PDF fail and says when the counts are partial", async () => {
    const corpus = {
      complete: false,
      sources: { qpdf: { files: 10, opened: 7, repaired: 1, refused: 0, refusedAlone: 0, errors: 2 } },
    };
    const files = section(await render({ ...PROOF, night: { ...NIGHT, corpus } }), "files");
    expect(files).toContain("2 files made Lamina PDF crash, hang or panic.");
    expect(files).toContain("so these counts cover only part of the run");
  });

  it("lists every allowed licence, exception and ban that deny.toml has", async () => {
    const licence = section(await render(PROOF), "licence");
    const policy = licencePolicy();
    expect(licence).toContain(`under ${policy.licence} :`);
    for (const allowed of policy.allowed) expect(licence).toContain(allowed);
    for (const exception of policy.exceptions) {
      expect(licence).toContain(`${exception.licence} , for ${exception.crates.join(" , ")}`);
    }
    for (const ban of policy.banned) expect(licence).toContain(`${ban.name} : ${ban.reason}`);
  });

  it("links to the website's pages only once they exist", async () => {
    const without = await render(PROOF, []);
    expect(without).not.toContain('href="/privacy/"');
    expect(without).toContain('href="https://docs.laminapdf.com/reference/benchmarks/"');
    const withPages = await render(PROOF, ["/src/pages/privacy.astro", "/src/pages/pricing/index.astro"]);
    expect(withPages).toContain('href="/privacy/"');
    expect(withPages).toContain('href="/pricing/"');
    expect(withPages).not.toContain('href="/performance/"');
  });
});

describe("percent", () => {
  it("never rounds up to 100% or down to 0%", () => {
    expect(percent(9999, 10000)).toBe("99.9%");
    expect(percent(1, 10000)).toBe("0.1%");
    expect(percent(10, 10)).toBe("100%");
    expect(percent(0, 10)).toBe("0%");
    expect(percent(0, 0)).toBe("–");
    expect(percent(5135, 6190)).toBe("83.0%");
  });
});
