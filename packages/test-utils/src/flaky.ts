// Browser suites retry a failed test once, and a test that passes on its retry is reported as
// flaky, never hidden (ADR 0018 §4, amended 2026-11-10). Real browsers on shared runners crash and
// stall for reasons a test can't control; the retry keeps those from failing a PR, and the report
// keeps every flaky test in sight until its issue is fixed.
//
// Vitest's own `github-actions` reporter already lists flaky tests in the job summary. This
// reporter adds what it lacks: a warning annotation per test, which shows on the run's page, and
// `flaky=true` in the step's outputs, which the nightly lane reads to open its issue.

import { appendFileSync } from "node:fs";
import { env as processEnv } from "node:process";
import type { InlineConfig, Reporter, TestCase } from "vitest/node";

type Env = Readonly<Record<string, string | undefined>>;

/** One flaky test: where it is, and the browser project it ran in. */
export interface FlakyTest {
  readonly project: string;
  readonly file: string;
  readonly name: string;
}

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

/**
 * A Vitest reporter for the tests that passed only on their retry: on GitHub Actions a warning
 * annotation each and `flaky=true` in the step's outputs, elsewhere a console warning.
 */
export function flakyReporter(env: Env = processEnv): Reporter {
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
      if (env.GITHUB_OUTPUT) appendFileSync(env.GITHUB_OUTPUT, "flaky=true\n");
    },
  };
}

/**
 * The `test` options every browser and e2e suite spreads in: one retry (more would hide a real
 * race), every test's line in the log, Vitest's GitHub reporter on Actions (failure annotations and
 * the job summary, flaky tests included), and the flaky reporter. `cargo xtask pdf-runtimes` passes
 * no `--reporter`, which would replace this list.
 */
export function retryOnceAndReport(env: Env = processEnv): Pick<InlineConfig, "retry" | "reporters"> {
  const github = env.GITHUB_ACTIONS === "true" ? ["github-actions"] : [];
  return { retry: 1, reporters: ["verbose", ...github, flakyReporter(env)] };
}
