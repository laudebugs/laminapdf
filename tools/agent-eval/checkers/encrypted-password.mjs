// open-secret.mjs: run three times (no password, a wrong one, the right one), each printing the
// expected JSON line and exiting 0; and it must not read the error's message text.
import { CheckFailure, check, expect, runNode, taskInput } from "./lib.mjs";

const PASSWORD = "user-pw";

/**
 * One run of the script: exit 0 and exactly the expected JSON.
 *
 * @param {string} project
 * @param {string[]} args
 * @param {object} want
 */
function expectRun(project, args, want) {
  const run = runNode(project, "open-secret.mjs", args);
  const label = `node open-secret.mjs ${args.join(" ")}`;
  expect(run.status === 0, `${label} exited ${run.status}: ${run.stderr.trim().split("\n").at(-1) ?? ""}`);
  const line = run.stdout.trim();
  let got;
  try {
    got = JSON.parse(line);
  } catch {
    throw new CheckFailure(`${label} printed ${JSON.stringify(line.slice(0, 80))}, not one JSON line`);
  }
  expect(JSON.stringify(got) === JSON.stringify(want), `${label} printed ${line}, expected ${JSON.stringify(want)}`);
}

await check(({ sdk, project, read }) => {
  const source = new TextDecoder().decode(read("open-secret.mjs"));
  expect(!/\.message\b/.test(source), "open-secret.mjs reads an error's .message instead of branching on its code");
  const pages = sdk.Document.open(taskInput("encrypted-password", "secret.pdf"), { password: PASSWORD }).pageCount;
  expectRun(project, ["secret.pdf"], { status: "password-required" });
  expectRun(project, ["secret.pdf", "not-the-password"], { status: "wrong-password" });
  expectRun(project, ["secret.pdf", PASSWORD], { status: "ok", pages });
  return "password-required, wrong-password and ok, each from the error code";
});
