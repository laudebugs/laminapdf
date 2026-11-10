// The flaky reporter (ADR 0018 §4): which tests it lists, what it writes on GitHub Actions and
// elsewhere, and the reporters the shared setting picks. Test cases are stand-ins with the three
// members the reporter reads.

import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describeFlaky, flakyReporter, retryOnceAndReport, warningLine } from "@laminapdf/test-utils/flaky";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { TestCase } from "vitest/node";

const PAGE = {
  project: "webkit",
  file: "test/browser/reader.browser.test.tsx",
  name: "PageInput > goes to a typed page",
};

function testCase(flaky: boolean | undefined): TestCase {
  const diagnostic = flaky === undefined ? undefined : { flaky };
  return {
    project: { name: PAGE.project },
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

function emptyFile(): string {
  const path = join(mkdtempSync(join(tmpdir(), "flaky-")), "output");
  writeFileSync(path, "");
  return path;
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("a flaky test's line", () => {
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
    expect(line).toBe(
      "::warning title=Flaky test (ADR 0018 §4)::[webkit] test/browser/reader.browser.test.tsx > 100%25 done%0Asecond line failed, then passed on its retry",
    );
  });
});

describe("the flaky reporter", () => {
  it("writes a warning per flaky test and flaky=true to the step's outputs on GitHub Actions", async () => {
    const output = emptyFile();
    const log = vi.spyOn(console, "log").mockImplementation(() => {});
    await run({ GITHUB_ACTIONS: "true", GITHUB_OUTPUT: output }, [testCase(true), testCase(false)]);
    expect(log.mock.calls.map(([line]) => line)).toEqual([warningLine(PAGE)]);
    expect(readFileSync(output, "utf8")).toBe("flaky=true\n");
  });

  it("warns on the console elsewhere", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    await run({}, [testCase(true)]);
    expect(warn).toHaveBeenCalledWith(`flaky: ${describeFlaky(PAGE)} failed, then passed on its retry`);
  });

  it("writes nothing when no test needed its retry", async () => {
    const output = emptyFile();
    const log = vi.spyOn(console, "log").mockImplementation(() => {});
    await run({ GITHUB_ACTIONS: "true", GITHUB_OUTPUT: output }, [testCase(false), testCase(undefined)]);
    expect(log).not.toHaveBeenCalled();
    expect(readFileSync(output, "utf8")).toBe("");
  });
});

describe("the shared setting", () => {
  it("retries a failed test once", () => {
    expect(retryOnceAndReport({}).retry).toBe(1);
  });

  it("keeps Vitest's GitHub reporter on Actions, beside the flaky one", () => {
    const [first, second, third] = retryOnceAndReport({ GITHUB_ACTIONS: "true" }).reporters ?? [];
    expect([first, second, typeof third]).toEqual(["verbose", "github-actions", "object"]);
  });

  it("leaves the GitHub reporter out elsewhere", () => {
    expect(retryOnceAndReport({}).reporters).toHaveLength(2);
  });
});
