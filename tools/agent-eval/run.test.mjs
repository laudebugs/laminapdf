// node --test tools/agent-eval/run.test.mjs: the runner's pure parts and the task files.
import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { test } from "node:test";
import { agentCommand, agentSummary, buildPlan, fullPrompt, loadConfig, parseArgs, renderMarkdown } from "./run.mjs";
import { loadTasks, parseInput, parseTask, ROOT } from "./tasks.mjs";

const TASK = `---
id: rotate
title: Rotate a page
checker: rotate.mjs
inputs: [fixtures/a.pdf => in.pdf, fixtures/b.pdf]
output: out.pdf
---
Rotate page 3.
`;

test("parseTask reads the front matter and the prompt", () => {
  assert.deepEqual(parseTask(TASK), {
    id: "rotate",
    title: "Rotate a page",
    checker: "rotate.mjs",
    inputs: [
      { from: "fixtures/a.pdf", to: "in.pdf" },
      { from: "fixtures/b.pdf", to: "b.pdf" },
    ],
    output: "out.pdf",
    prompt: "Rotate page 3.",
  });
});

test("parseTask accepts an empty input list and refuses a missing field or prompt", () => {
  assert.deepEqual(parseTask(TASK.replace(/inputs: .*/, "inputs: []")).inputs, []);
  assert.throws(() => parseTask(TASK.replace(/checker: .*\n/, ""), "x.md"), /x\.md: missing "checker"/);
  assert.throws(() => parseTask(TASK.replace("Rotate page 3.", "")), /empty prompt/);
  assert.throws(() => parseTask("no front matter"), /no front matter/);
  assert.throws(() => parseInput("=> b.pdf"), /empty input/);
});

test("every task file parses, names its checker and inputs, and both exist", () => {
  const tasks = loadTasks();
  assert.ok(tasks.length >= 10 && tasks.length <= 15, `${tasks.length} tasks`);
  for (const task of tasks) {
    assert.ok(existsSync(new URL(`checkers/${task.checker}`, import.meta.url)), `${task.id}: no checker`);
    for (const input of task.inputs) assert.ok(existsSync(new URL(input.from, ROOT)), `${task.id}: no ${input.from}`);
  }
});

test("parseArgs reads the task and mode lists and refuses unknown ones", () => {
  assert.deepEqual(parseArgs([]), { tasks: undefined, modes: ["no-docs", "docs"], dryRun: false, keep: false });
  assert.deepEqual(parseArgs(["--tasks", "a,b", "--modes", "docs", "--dry-run"]), {
    tasks: ["a", "b"],
    modes: ["docs"],
    dryRun: true,
    keep: false,
  });
  assert.throws(() => parseArgs(["--modes", "some-docs"]), /unknown mode some-docs/);
  assert.throws(() => parseArgs(["--tasks"]), /--tasks needs/);
  assert.throws(() => parseArgs(["--fast"]), /unknown argument/);
});

test("buildPlan runs every chosen task in every mode, task by task", () => {
  const tasks = ["a", "b", "c"].map((id) => parseTask(TASK.replace("id: rotate", `id: ${id}`)));
  const plan = buildPlan(tasks, { tasks: ["c", "a"], modes: ["no-docs", "docs"] });
  assert.deepEqual(
    plan.map((s) => `${s.task.id}/${s.mode}`),
    ["a/no-docs", "a/docs", "c/no-docs", "c/docs"],
  );
  assert.equal(buildPlan(tasks, { tasks: undefined, modes: ["docs"] }).length, 3);
  assert.throws(() => buildPlan(tasks, { tasks: ["d"], modes: ["docs"] }), /unknown task d/);
});

test("agentCommand pins the agent and model and keeps the variadic flag last", () => {
  const config = loadConfig();
  const { command, args } = agentCommand(config);
  assert.equal(command, "npx");
  assert.equal(args[1], `${config.agent.package}@${config.agent.version}`);
  assert.match(config.agent.version, /^\d+\.\d+\.\d+$/, "an exact version");
  assert.equal(args[args.indexOf("--model") + 1], config.model);
  assert.equal(args[args.indexOf("--max-turns") + 1], String(config.maxTurns));
  assert.deepEqual(args.slice(args.indexOf("--disallowedTools") + 1), config.agent.disallowedTools);
  assert.ok(!args.some((a) => a.includes("sk-ant")), "no key on the command line");
});

test("fullPrompt puts the preamble first", () => {
  const config = { ...loadConfig(), preamble: "Use node." };
  assert.equal(fullPrompt(config, parseTask(TASK)), "Use node.\n\nRotate page 3.\n");
});

test("agentSummary reads Claude Code's JSON result and survives anything else", () => {
  const json = JSON.stringify({ type: "result", subtype: "success", num_turns: 7, total_cost_usd: 0.42 });
  assert.deepEqual(agentSummary(`${json}\n`), { turns: 7, costUsd: 0.42, subtype: "success" });
  assert.deepEqual(agentSummary("not json"), {});
});

test("renderMarkdown shows a task × mode summary and one row per run", () => {
  const results = [
    { task: "a", mode: "no-docs", pass: false, reason: "x | y", durationMs: 61_400, turns: 9, costUsd: 0.5 },
    { task: "a", mode: "docs", pass: true, reason: "ok", durationMs: 30_000, turns: 4, costUsd: undefined },
  ];
  const md = renderMarkdown({ date: "2026-11-04", model: "m" }, /** @type {any} */ (results));
  assert.match(md, /^# Agent eval, 2026-11-04\n\n- date: 2026-11-04\n- model: m\n/);
  assert.match(
    md,
    /\| Task \| no-docs \| docs \|\n\| --- \| --- \| --- \|\n\| a \| fail \| pass \|\n\| Passed \| \*\*0\/1\*\* \| \*\*1\/1\*\* \|/,
  );
  assert.match(md, /\| a \| no-docs \| fail \| x \\\| y \| 61 s \| 9 \| \$0\.50 \|/);
  assert.match(md, /\| a \| docs \| pass \| ok \| 30 s \| 4 \| — \|/);
});
