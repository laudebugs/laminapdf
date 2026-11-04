# Agent eval

This eval measures whether a coding agent writes working Lamina PDF code more often when it can read the docs that `@laminapdf/sdk` ships. A pinned agent gets twelve fixed tasks, each run twice in a fresh project: once with the package's `docs/` folder deleted, and once with the docs and the `AGENTS.md` snippet in place. A checker then judges each result with Lamina PDF itself and `qpdf --check`. The eval is recorded by hand and never gates anything: it needs a paid API key, and an agent's answers vary from run to run.

## What a run does

For each task in `tasks/` and each mode, `run.mjs`:

1. copies a project with the packed `@laminapdf/wasm` and `@laminapdf/sdk` installed into a new temp folder;
2. in `no-docs` mode, deletes `node_modules/@laminapdf/sdk/docs` and writes no `AGENTS.md`; in `docs` mode, keeps the docs and copies `packages/sdk/examples/AGENTS.md` to the project's `AGENTS.md` and `CLAUDE.md`;
3. copies the task's input files from `fixtures/`;
4. starts the agent pinned in `eval.config.json` in print mode, with the task's prompt on stdin;
5. runs `checkers/<task>.mjs` on the project, which prints `PASS: …` or `FAIL: <reason>`.

Both modes get the same prompt, the same package and the same agent settings. Nothing in the project points back to this repository: the tarballs are installed from temporary copies, which are deleted afterwards, and `package.json` names plain versions. The agent runs with web search and web fetch turned off, so neither mode can read the docs online, and with an empty config folder, so your own Claude Code settings, memory and MCP servers stay out of it.

The tasks cover the first things people do with the library: build a page with text at a given position, a two-page invoice with a table, metadata, links and an outline, a tagged PDF/UA-1 document, and an image with a caption. Others open an encrypted file and branch on `wrong-password`, rotate a page and save incrementally, edit metadata incrementally, list page sizes, extract text, and repair a damaged file. Each task file holds its prompt and front matter (`id`, `title`, `checker`, `inputs`, `output`).

## Run it

You need Node 22 or later, npm, and an Anthropic API key. Install qpdf as well: without it, the checkers skip `qpdf --check` and say so in each reason.

```sh
cargo xtask publish npm-pack                   # packs @laminapdf/wasm and @laminapdf/sdk into target/xtask/npm-packs/
export ANTHROPIC_API_KEY=…                     # read from the environment only; never put it in a file in the repository
node tools/agent-eval/run.mjs --dry-run        # the plan and the agent's command line, without running anything
node tools/agent-eval/run.mjs                  # every task in both modes
node tools/agent-eval/run.mjs --tasks rotate-page-3,extract-text --modes docs
```

`--dry-run` needs neither the tarballs nor the key: it prints the plan and then says what a real run would lack. `--keep` leaves the scratch projects in the temp folder for inspection. Runs go one at a time, and each is limited by `timeoutSeconds`, `maxTurns` and `maxBudgetUsd` in `eval.config.json`.

## Cost

A full run is 24 agent sessions. With a cap of 2 US dollars per session, a full run costs at most 48 dollars; a session that finishes its task early costs less. Each result row records its cost, read from the agent's JSON output. While you work on a task or a checker, run only that task with `--tasks` and one mode with `--modes`.

## Results

Each run writes to `target/agent-eval/<date>/` (`<date>-2` and so on for a second run that day):

- `results.json`: the agent, model, Lamina PDF version, Node and qpdf versions, and one entry per task and mode (pass, reason, agent time, turns, cost);
- `results.md`: the same as a task × mode table, then one row per run;
- `logs/<task>/<mode>/`: the agent's JSON output and stderr, and the files it left in its project (without `node_modules`).

## Record a baseline

The first full run is the baseline that later changes to names and docs are measured against. To record it:

1. run every task in both modes on a clean checkout of `main`, after `cargo xtask publish npm-pack`;
2. copy the summary table and the details from `results.md` into [the baseline page](../../docs/research/libraries/agent-eval-baseline.md), with the agent, model, date and Lamina PDF version from its header;
3. open a docs PR with the page.

Keep the task list, the checkers and `eval.config.json` unchanged between runs you want to compare. A change to any of them starts a new baseline: say so on the page.

## Change the eval

To add a task, write `tasks/<id>.md` and `checkers/<id>.mjs`. The checker calls `check()` from `checkers/lib.mjs`, which loads the project's own `@laminapdf/sdk`, gives the checks helpers to read and open files, and runs `qpdf --check` on every file the checks mark with `pdf()`. A check that compares with an input reads the fixture through `taskInput()`, so an agent that edits its input in place can't change what the checker compares. `node --test tools/agent-eval/run.test.mjs` checks the runner and that every task's checker and inputs exist.

To change the agent or its flags, edit `eval.config.json` and `agentCommand()` in `run.mjs`, the one function that builds the command line. The flags match Claude Code 2.1.316's `--help`. `--max-turns` is a hidden flag that the CLI still accepts in print mode, and `--disallowedTools` takes several values, so it comes last.
