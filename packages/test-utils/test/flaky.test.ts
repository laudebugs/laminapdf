// The flaky reporter (ADR 0018 §4): which tests it lists, and what it writes on GitHub Actions and
// elsewhere. Test cases are stand-ins with the three members the reporter reads.

import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  describeFlaky,
  flakyReporter,
  RETRIES,
  retryOnceAndReport,
  summaryMarkdown,
  warningLine,
} from "@laminapdf/test-utils/flaky";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { TestCase } from "vitest/node";

const PAGE = {
  project: "webkit",
  file: "test/browser/reader.browser.test.tsx",
  name: "PageInput > goes to a typed page",
};

function testCase(flaky: boolean | undefined, project = "webkit"): TestCase {
  const diagnostic = flaky === undefined ? undefined : { flaky };
  return {
    project: { name: project },
    module: { relativeModuleId: PAGE.file },
    fullName: PAGE.name,
    diagnostic: () => diagnostic,
  } as unknown as TestCase;
}

async function run(env: Record<string, string>, cases: TestCase[]): Promise<void> {
  const reporter = flakyReporter(env);
  for (const test of cases) await reporter.onTestCaseResult?.(test);
  await reporter.onTestRunEnd?.([], [], "passed");
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("the flaky report", () => {
  it("names the browser, the file and the test", () => {
    expect(describeFlaky(PAGE)).toBe(
      "[webkit] test/browser/reader.browser.test.tsx > PageInput > goes to a typed page",
    );
  });

  it("leaves the browser out when the project has no name", () => {
    expect(describeFlaky({ ...PAGE, project: "" })).toBe(`${PAGE.file} > ${PAGE.name}`);
  });

  it("escapes what a workflow command would read as its own syntax", () => {
    const line = warningLine({ ...PAGE, name: "100% done\nsecond line" });
    expect(line).toContain("100%25 done%0Asecond line");
    expect(line.startsWith("::warning title=Flaky test (ADR 0018 §4)::")).toBe(true);
  });

  it("lists each test once in the summary, as code", () => {
    expect(summaryMarkdown([PAGE])).toContain(`- \`${describeFlaky(PAGE)}\``);
  });

  it("retries a failed test once, and no more", () => {
    expect(RETRIES).toBe(1);
    expect(retryOnceAndReport().retry).toBe(1);
  });
});

describe("the flaky reporter", () => {
  it("writes a warning per flaky test and a summary section on GitHub Actions", async () => {
    const summary = join(mkdtempSync(join(tmpdir(), "flaky-")), "summary.md");
    writeFileSync(summary, "");
    const log = vi.spyOn(console, "log").mockImplementation(() => {});
    await run({ GITHUB_ACTIONS: "true", GITHUB_STEP_SUMMARY: summary }, [testCase(true), testCase(false)]);
    expect(log.mock.calls.map(([line]) => line)).toEqual([warningLine(PAGE)]);
    expect(readFileSync(summary, "utf8")).toBe(summaryMarkdown([PAGE]));
  });

  it("warns on the console elsewhere", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    await run({}, [testCase(true)]);
    expect(warn).toHaveBeenCalledWith(`flaky: ${describeFlaky(PAGE)} failed, then passed on its retry`);
  });

  it("says nothing when no test needed its retry", async () => {
    const log = vi.spyOn(console, "log").mockImplementation(() => {});
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    await run({ GITHUB_ACTIONS: "true" }, [testCase(false), testCase(undefined)]);
    expect(log).not.toHaveBeenCalled();
    expect(warn).not.toHaveBeenCalled();
  });
});
