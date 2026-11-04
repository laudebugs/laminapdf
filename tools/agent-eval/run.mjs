// The agent eval (todo L5a): does a coding agent write working Lamina PDF code more often with
// the docs @laminapdf/sdk ships than without them? For each task in tasks/ and each mode, the
// runner makes a scratch project in the temp folder, installs the packed @laminapdf/wasm and
// @laminapdf/sdk, starts the pinned agent (eval.config.json) on the task's prompt, then runs the
// task's checker. Recorded, never gating, never in CI: it needs a paid API key.
//
//   node tools/agent-eval/run.mjs [--tasks a,b] [--modes docs,no-docs] [--keep] [--dry-run]
//
// Modes: `no-docs` deletes node_modules/@laminapdf/sdk/docs and writes no AGENTS.md; `docs`
// keeps them and writes packages/sdk/examples/AGENTS.md to the project's AGENTS.md and CLAUDE.md.
// Results: target/agent-eval/<date>/results.json and results.md, logs and each project's files
// (without node_modules) under logs/<task>/<mode>/.
import { spawnSync } from "node:child_process";
import {
  copyFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import process from "node:process";
import { fileURLToPath, pathToFileURL } from "node:url";
import { loadTasks, ROOT } from "./tasks.mjs";

/** @typedef {import("./tasks.mjs").Task} Task */
/** @typedef {"no-docs" | "docs"} Mode */
/**
 * @typedef {object} Config
 * @property {{ name: string, package: string, version: string, disallowedTools: string[] }} agent
 * @property {string} model
 * @property {number} maxTurns
 * @property {number} maxBudgetUsd
 * @property {number} timeoutSeconds
 * @property {number} checkerTimeoutSeconds
 * @property {string} packs - the folder `cargo xtask publish npm-pack` writes, relative to the root
 * @property {string} preamble - put before every task's prompt
 *
 * @typedef {object} Options
 * @property {string[] | undefined} tasks
 * @property {Mode[]} modes
 * @property {boolean} dryRun
 * @property {boolean} keep - keep the scratch projects
 *
 * @typedef {object} Result
 * @property {string} task
 * @property {Mode} mode
 * @property {boolean} pass
 * @property {string} reason
 * @property {number} durationMs - the agent's wall-clock time
 * @property {number | undefined} turns
 * @property {number | undefined} costUsd
 */

/** @type {readonly Mode[]} */
export const MODES = ["no-docs", "docs"];
const HERE = fileURLToPath(new URL(".", import.meta.url));
const ROOT_DIR = fileURLToPath(ROOT);

/**
 * The command line's options.
 *
 * @param {string[]} argv - the arguments after the script
 * @returns {Options}
 */
export function parseArgs(argv) {
  /** @type {Options} */
  const options = { tasks: undefined, modes: [...MODES], dryRun: false, keep: false };
  const list = (/** @type {string | undefined} */ v, /** @type {string} */ flag) => {
    if (!v || v.startsWith("--")) throw new Error(`${flag} needs a comma-separated list`);
    return v.split(",").filter((s) => s !== "");
  };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--tasks") options.tasks = list(argv[++i], arg);
    else if (arg === "--modes") options.modes = /** @type {Mode[]} */ (list(argv[++i], arg));
    else if (arg === "--dry-run") options.dryRun = true;
    else if (arg === "--keep") options.keep = true;
    else throw new Error(`unknown argument: ${arg}`);
  }
  const unknown = options.modes.filter((m) => !MODES.includes(m));
  if (unknown.length > 0) throw new Error(`unknown mode ${unknown.join(", ")}: use ${MODES.join(", ")}`);
  return options;
}

/**
 * Every (task, mode) pair to run, task by task, in the order of `options.modes`.
 *
 * @param {Task[]} tasks
 * @param {Pick<Options, "tasks" | "modes">} options
 * @returns {{ task: Task, mode: Mode }[]}
 */
export function buildPlan(tasks, options) {
  const ids = new Set(tasks.map((t) => t.id));
  const missing = (options.tasks ?? []).filter((id) => !ids.has(id));
  if (missing.length > 0) throw new Error(`unknown task ${missing.join(", ")}`);
  const chosen = options.tasks ? tasks.filter((t) => options.tasks?.includes(t.id)) : tasks;
  return chosen.flatMap((task) => options.modes.map((mode) => ({ task, mode })));
}

/**
 * The prompt the agent gets: the preamble, then the task's own prompt.
 *
 * @param {Config} config
 * @param {Task} task
 */
export function fullPrompt(config, task) {
  return `${config.preamble}\n\n${task.prompt}\n`;
}

/**
 * The agent's command line, run in the scratch project with the prompt on stdin (Claude Code's
 * print mode reads it there when no prompt argument is given). The one place to adjust when the
 * agent or its flags change. The variadic `--disallowedTools` goes last, so it can't swallow
 * another argument; web tools are off so that no mode can read the docs online.
 *
 * @param {Config} config
 * @returns {{ command: string, args: string[] }}
 */
export function agentCommand(config) {
  const { agent } = config;
  return {
    command: "npx",
    args: [
      "-y",
      `${agent.package}@${agent.version}`,
      "--print",
      "--model",
      config.model,
      "--max-turns",
      String(config.maxTurns),
      "--max-budget-usd",
      String(config.maxBudgetUsd),
      "--output-format",
      "json",
      "--dangerously-skip-permissions",
      "--no-session-persistence",
      "--setting-sources",
      "project",
      "--strict-mcp-config",
      "--disallowedTools",
      ...agent.disallowedTools,
    ],
  };
}

/**
 * The environment the agent runs with: an empty config folder, so the user's own settings,
 * memory and MCP servers stay out, and no auto-update.
 *
 * @param {string} project
 */
function agentEnv(project) {
  return { ...process.env, CLAUDE_CONFIG_DIR: join(project, ".claude-home"), DISABLE_AUTOUPDATER: "1" };
}

/**
 * The packed tarballs, or an error saying how to make them.
 *
 * @param {Config} config
 * @returns {{ wasm: string, sdk: string, version: string }}
 */
export function findPacks(config) {
  const dir = join(ROOT_DIR, config.packs);
  // The checkout's version names the tarballs, so a stale pack of another version is never used.
  const { version } = JSON.parse(readFileSync(join(ROOT_DIR, "packages/sdk/package.json"), "utf8"));
  const wasm = join(dir, `laminapdf-wasm-${version}.tgz`);
  const sdk = join(dir, `laminapdf-sdk-${version}.tgz`);
  if (!existsSync(wasm) || !existsSync(sdk)) {
    throw new Error(
      `no @laminapdf/wasm and @laminapdf/sdk ${version} tarballs in ${config.packs}: run \`cargo xtask publish npm-pack\` first`,
    );
  }
  return { wasm, sdk, version };
}

/**
 * Runs a command to the end; throws with its output when it fails.
 *
 * @param {string} command
 * @param {string[]} args
 * @param {string} cwd
 */
function run(command, args, cwd) {
  const r = spawnSync(command, args, { cwd, encoding: "utf8" });
  if (r.status !== 0) throw new Error(`${command} ${args.join(" ")} failed (${r.status}):\n${r.stdout}${r.stderr}`);
  return r.stdout;
}

/**
 * The scratch project's package.json.
 *
 * @param {Record<string, string>} [dependencies]
 */
function packageJson(dependencies) {
  const json = { name: "agent-eval-task", private: true, type: "module", ...(dependencies ? { dependencies } : {}) };
  return `${JSON.stringify(json, null, 2)}\n`;
}

/**
 * A project with the tarballs installed, made once per mode and copied for each task. The
 * tarballs sit in the project's `vendor/` and package.json names them with `file:`, so an
 * `npm install` the agent runs reinstalls the same files offline, never the registry's version
 * and never a path into this repository. In `no-docs` mode the sdk tarball is repacked without
 * its `docs/`, so a reinstall can't bring the docs back.
 *
 * @param {{ wasm: string, sdk: string, version: string }} packs
 * @param {Mode} mode
 * @param {string} base - the run's temp folder
 */
export function makeTemplate(packs, mode, base) {
  const dir = join(base, `template-${mode}`);
  const vendor = join(dir, "vendor");
  mkdirSync(vendor, { recursive: true });
  const wasm = join(vendor, "laminapdf-wasm.tgz");
  const sdk = join(vendor, "laminapdf-sdk.tgz");
  copyFileSync(packs.wasm, wasm);
  if (mode === "no-docs") withoutDocs(packs.sdk, sdk, join(base, "repack"));
  else copyFileSync(packs.sdk, sdk);
  writeFileSync(
    join(dir, "package.json"),
    packageJson({
      "@laminapdf/sdk": "file:vendor/laminapdf-sdk.tgz",
      "@laminapdf/wasm": "file:vendor/laminapdf-wasm.tgz",
    }),
  );
  run("npm", ["install", "--no-audit", "--no-fund", "--offline", "--loglevel=error"], dir);
  const docs = join(dir, "node_modules", "@laminapdf", "sdk", "docs");
  if (mode === "no-docs" && existsSync(docs)) throw new Error("the repacked @laminapdf/sdk still has docs/");
  if (mode === "docs" && !existsSync(docs)) throw new Error("the packed @laminapdf/sdk has no docs/ folder: repack it");
  return dir;
}

/**
 * Repacks the tarball `from` into `to` without `package/docs/`, in the scratch folder `work`.
 *
 * @param {string} from
 * @param {string} to
 * @param {string} work
 */
function withoutDocs(from, to, work) {
  rmSync(work, { recursive: true, force: true });
  mkdirSync(work, { recursive: true });
  run("tar", ["-xzf", from, "-C", work], work);
  rmSync(join(work, "package", "docs"), { recursive: true, force: true });
  run("tar", ["-czf", to, "-C", work, "package"], work);
  rmSync(work, { recursive: true, force: true });
}

/**
 * A fresh scratch project for one task and mode: the template, the AGENTS.md snippet in `docs`
 * mode, and the task's inputs.
 *
 * @param {string} template
 * @param {Task} task
 * @param {Mode} mode
 * @param {string} base
 */
function makeProject(template, task, mode, base) {
  const dir = join(base, `${task.id}-${mode}`);
  cpSync(template, dir, { recursive: true });
  if (mode === "docs") {
    const snippet = join(ROOT_DIR, "packages", "sdk", "examples", "AGENTS.md");
    for (const name of ["AGENTS.md", "CLAUDE.md"]) copyFileSync(snippet, join(dir, name));
  }
  for (const input of task.inputs) copyFileSync(join(ROOT_DIR, input.from), join(dir, input.to));
  return dir;
}

/**
 * The fields of Claude Code's `--output-format json` result that the report keeps.
 *
 * @param {string} stdout
 * @returns {{ turns?: number, costUsd?: number, subtype?: string }}
 */
export function agentSummary(stdout) {
  try {
    const json = JSON.parse(stdout.trim().split("\n").at(-1) ?? "");
    return { turns: json.num_turns, costUsd: json.total_cost_usd, subtype: json.subtype };
  } catch {
    return {};
  }
}

/**
 * Runs the agent in `project`; its output goes to `logs`.
 *
 * @param {Config} config
 * @param {Task} task
 * @param {string} project
 * @param {string} logs
 */
function runAgent(config, task, project, logs) {
  const { command, args } = agentCommand(config);
  const started = Date.now();
  const r = spawnSync(command, args, {
    cwd: project,
    env: agentEnv(project),
    input: fullPrompt(config, task),
    encoding: "utf8",
    timeout: config.timeoutSeconds * 1000,
    maxBuffer: 64 * 1024 * 1024,
  });
  const durationMs = Date.now() - started;
  writeFileSync(join(logs, "agent.json"), r.stdout ?? "");
  writeFileSync(join(logs, "agent.stderr.log"), r.stderr ?? "");
  const timedOut = r.error && /** @type {NodeJS.ErrnoException} */ (r.error).code === "ETIMEDOUT";
  return { durationMs, timedOut: Boolean(timedOut), status: r.status, ...agentSummary(r.stdout ?? "") };
}

/**
 * Runs the task's checker on `project`: its verdict and one-line reason.
 *
 * @param {Config} config
 * @param {Task} task
 * @param {string} project
 */
function runChecker(config, task, project) {
  const checker = join(HERE, "checkers", task.checker);
  const r = spawnSync(process.execPath, [checker, project], {
    encoding: "utf8",
    timeout: config.checkerTimeoutSeconds * 1000,
  });
  const line = (r.stdout ?? "").trim().split("\n").at(-1) ?? "";
  const reason = line.replace(/^(PASS|FAIL): /, "") || (r.stderr ?? "").trim().split("\n").at(-1) || "no output";
  return { pass: r.status === 0, reason: r.error ? `checker didn't finish: ${r.error.message}` : reason };
}

/**
 * Copies what the agent left in the project (without node_modules and the agent's config) to
 * the logs, so a failure can be read after the scratch project is gone.
 *
 * @param {string} project
 * @param {string} logs
 */
function keepFiles(project, logs) {
  const skip = new Set(["node_modules", ".claude-home"]);
  cpSync(project, join(logs, "project"), {
    recursive: true,
    filter: (src) => !src.split(/[\\/]/).some((p) => skip.has(p)),
  });
}

/**
 * One task in one mode, from a fresh project to the checker's verdict.
 *
 * @param {Config} config
 * @param {{ task: Task, mode: Mode }} step
 * @param {{ templates: Map<Mode, string>, base: string, out: string, keep: boolean }} ctx
 * @returns {Result}
 */
function runStep(config, { task, mode }, ctx) {
  const logs = join(ctx.out, "logs", task.id, mode);
  mkdirSync(logs, { recursive: true });
  const project = makeProject(/** @type {string} */ (ctx.templates.get(mode)), task, mode, ctx.base);
  const agent = runAgent(config, task, project, logs);
  const outcome = agent.timedOut
    ? { pass: false, reason: `the agent timed out after ${config.timeoutSeconds} s` }
    : runChecker(config, task, project);
  keepFiles(project, logs);
  if (!ctx.keep) rmSync(project, { recursive: true, force: true });
  return { task: task.id, mode, ...outcome, durationMs: agent.durationMs, turns: agent.turns, costUsd: agent.costUsd };
}

/** A table cell: no pipes or line breaks. */
function cell(/** @type {string} */ text) {
  return text.replaceAll("|", "\\|").replaceAll("\n", " ");
}

/**
 * The results as Markdown: a task × mode summary, then one row per run.
 *
 * @param {Record<string, string>} meta - shown above the tables (agent, model, version, …)
 * @param {Result[]} results
 * @param {readonly Mode[]} [modes]
 * @returns {string}
 */
export function renderMarkdown(meta, results, modes = MODES) {
  const used = modes.filter((m) => results.some((r) => r.mode === m));
  const tasks = [...new Set(results.map((r) => r.task))];
  const find = (/** @type {string} */ t, /** @type {Mode} */ m) => results.find((r) => r.task === t && r.mode === m);
  const passed = (/** @type {Mode} */ m) => {
    const runs = results.filter((r) => r.mode === m);
    return `**${runs.filter((r) => r.pass).length}/${runs.length}**`;
  };
  const lines = [`# Agent eval, ${meta.date}`, "", ...Object.entries(meta).map(([k, v]) => `- ${k}: ${v}`), ""];
  lines.push(`| Task | ${used.join(" | ")} |`, `| --- |${" --- |".repeat(used.length)}`);
  for (const t of tasks) lines.push(`| ${t} | ${used.map((m) => verdict(find(t, m))).join(" | ")} |`);
  lines.push(`| Passed | ${used.map(passed).join(" | ")} |`, "");
  lines.push(
    "| Task | Mode | Result | Reason | Agent time | Turns | Cost |",
    "| --- | --- | --- | --- | --- | --- | --- |",
  );
  for (const r of results) lines.push(detailRow(r));
  return `${lines.join("\n")}\n`;
}

/** @param {Result | undefined} r */
function verdict(r) {
  return r === undefined ? "—" : r.pass ? "pass" : "fail";
}

/** @param {Result} r */
function detailRow(r) {
  const seconds = `${Math.round(r.durationMs / 1000)} s`;
  const cost = r.costUsd === undefined ? "—" : `$${r.costUsd.toFixed(2)}`;
  return `| ${r.task} | ${r.mode} | ${verdict(r)} | ${cell(r.reason)} | ${seconds} | ${r.turns ?? "—"} | ${cost} |`;
}

/**
 * target/agent-eval/<date>/, or <date>-2, -3 … when a run that day already wrote one.
 *
 * @param {string} date - YYYY-MM-DD
 */
export function outputDir(date) {
  const base = join(ROOT_DIR, "target", "agent-eval");
  let dir = join(base, date);
  for (let n = 2; existsSync(dir); n++) dir = join(base, `${date}-${n}`);
  return dir;
}

/** The qpdf version, or a note that it is missing (the checkers then skip `qpdf --check`). */
function qpdfVersion() {
  const r = spawnSync("qpdf", ["--version"], { encoding: "utf8" });
  return r.error ? "not installed (checks skipped)" : (r.stdout.split("\n")[0] ?? "").replace(/^qpdf version /, "");
}

/**
 * Prints the plan and the commands without running anything, and what a real run would lack.
 *
 * @param {Config} config
 * @param {{ task: Task, mode: Mode }[]} plan
 */
function printDryRun(config, plan) {
  const { command, args } = agentCommand(config);
  console.log(`agent: ${command} ${args.join(" ")} < prompt`);
  console.log(`checker: node tools/agent-eval/checkers/<checker> <project>\n`);
  for (const { task, mode } of plan) {
    const inputs = task.inputs.map((i) => i.to).join(", ") || "none";
    console.log(
      `${task.id} [${mode}]: inputs ${inputs}; output ${task.output}; checker ${task.checker}; prompt ${fullPrompt(config, task).length} chars`,
    );
  }
  console.log(`\n${plan.length} runs.`);
  try {
    console.log(`packs: ${findPacks(config).sdk}`);
  } catch (e) {
    console.log(`packs: ${e instanceof Error ? e.message : e}`);
  }
  console.log(`ANTHROPIC_API_KEY: ${process.env.ANTHROPIC_API_KEY ? "set" : "not set (a real run refuses to start)"}`);
  console.log(`qpdf: ${qpdfVersion()}`);
}

/**
 * Runs the plan and writes results.json and results.md.
 *
 * @param {Config} config
 * @param {{ task: Task, mode: Mode }[]} plan
 * @param {Options} options
 */
function runPlan(config, plan, options) {
  if (!process.env.ANTHROPIC_API_KEY) throw new Error("ANTHROPIC_API_KEY is not set: export it (never commit it)");
  const packs = findPacks(config);
  const date = new Date().toISOString().slice(0, 10);
  const out = outputDir(date);
  const base = mkdtempSync(join(tmpdir(), "lamina-agent-eval-"));
  const modes = [...new Set(plan.map((s) => s.mode))];
  const templates = new Map(modes.map((m) => [m, makeTemplate(packs, m, base)]));
  run("npx", ["-y", `${config.agent.package}@${config.agent.version}`, "--version"], base);
  /** @type {Result[]} */
  const results = [];
  for (const step of plan) {
    console.log(`${step.task.id} [${step.mode}] …`);
    results.push(runStep(config, step, { templates, base, out, keep: options.keep }));
    console.log(`  ${verdict(results.at(-1))}: ${results.at(-1)?.reason}`);
  }
  const meta = {
    date,
    agent: `${config.agent.name} ${config.agent.version}`,
    model: config.model,
    laminapdf: packs.version,
    node: process.version,
    qpdf: qpdfVersion(),
    "max turns": String(config.maxTurns),
  };
  writeFileSync(join(out, "results.json"), `${JSON.stringify({ meta, results }, null, 2)}\n`);
  writeFileSync(join(out, "results.md"), renderMarkdown(meta, results, options.modes));
  if (!options.keep) rmSync(base, { recursive: true, force: true });
  console.log(`\nresults: ${join(out, "results.md")}`);
}

/** Reads eval.config.json. */
export function loadConfig() {
  return /** @type {Config} */ (JSON.parse(readFileSync(join(HERE, "eval.config.json"), "utf8")));
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const config = loadConfig();
  const plan = buildPlan(loadTasks(), options);
  if (options.dryRun) printDryRun(config, plan);
  else runPlan(config, plan, options);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main();
  } catch (e) {
    console.error(`agent-eval: ${e instanceof Error ? e.message : e}`);
    process.exit(1);
  }
}
