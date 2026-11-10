// Browser suites retry a failed test once, and a test that passes on its retry is reported as
// flaky, never hidden (ADR 0018 §4, amended 2026-11-10). Real browsers on shared runners crash and
// stall for reasons a test can't control; the retry keeps those from failing a PR, and the report
// keeps every flaky test in sight until its issue is fixed.

import { appendFileSync } from "node:fs";
import { env as processEnv } from "node:process";
import type { InlineConfig, Reporter, TestCase } from "vitest/node";

/** One flaky test: where it is, and the browser project it ran in. */
export interface FlakyTest {
  readonly project: string;
  readonly file: string;
  readonly name: string;
}

/** A failed test runs once more; more retries would hide a real race. */
export const RETRIES = 1;

/** The test as one line: `[webkit] test/browser/reader.browser.test.tsx > PageInput > goes to a typed page`. */
export function describeFlaky(test: FlakyTest): string {
  const project = test.project === "" ? "" : `[${test.project}] `;
  return `${project}${test.file} > ${test.name}`;
}

/** A GitHub Actions warning for the test, with the command's special characters escaped. */
export function warningLine(test: FlakyTest): string {
  const message = describeFlaky(test).replaceAll("%", "%25").replaceAll("\r", "%0D").replaceAll("\n", "%0A");
  return `::warning title=Flaky test (ADR 0018 §4)::${message} failed, then passed on its retry`;
}

/** The job summary's section: a heading and one bullet per test. */
export function summaryMarkdown(tests: readonly FlakyTest[]): string {
  const rows = tests.map((test) => `- \`${describeFlaky(test).replaceAll("`", "'")}\``);
  return `### Flaky tests\n\nThese failed, then passed on their retry (ADR 0018 §4). Each one is a bug: open or update its issue.\n\n${rows.join("\n")}\n`;
}

/**
 * A Vitest reporter that lists every test that passed only on its retry: a warning annotation per
 * test and a section in the job summary on GitHub Actions, a console warning elsewhere.
 */
export function flakyReporter(env: Readonly<Record<string, string | undefined>> = processEnv): Reporter {
  const flaky: FlakyTest[] = [];
  return {
    onTestCaseResult(test: TestCase) {
      if (test.diagnostic()?.flaky !== true) return;
      flaky.push({ project: test.project.name, file: test.module.relativeModuleId, name: test.fullName });
    },
    onTestRunEnd() {
      if (flaky.length === 0) return;
      if (env.GITHUB_ACTIONS !== "true") {
        for (const test of flaky) console.warn(`flaky: ${describeFlaky(test)} failed, then passed on its retry`);
        return;
      }
      for (const test of flaky) console.log(warningLine(test));
      if (env.GITHUB_STEP_SUMMARY) appendFileSync(env.GITHUB_STEP_SUMMARY, summaryMarkdown(flaky));
    },
  };
}

/** The `test` options every browser suite spreads in: one retry, and the flaky report. */
export function retryOnceAndReport(): Pick<InlineConfig, "retry" | "reporters"> {
  return { retry: RETRIES, reporters: ["default", flakyReporter()] };
}
