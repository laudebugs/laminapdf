# L5b — Agent-eval baseline: run the agent eval with and without the shipped docs and record it

| Field | Value |
| --- | --- |
| ID | `L5b` |
| Status | `ready` |
| Plan | [shareable-libraries §7 L5](../../plans/shareable-libraries.md#l5-docs-for-agents) |
| Round / milestone | Round L5 (shareable libraries) |
| Depends on | [L5a](../done/L5a-agent-docs.md) |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md) (the eval is recorded, not gating) |
| Branch | `docs/l5b-agent-eval-baseline` |
| PR title | `docs(research): record the agent-eval baseline` |
| Size | S (a results page; the tool exists) |
| PR | — |

## Goal

Find out whether the docs the packages ship help a coding agent write working Lamina PDF code. [L5a](../done/L5a-agent-docs.md) built the eval (`tools/agent-eval/`) but could not run it: a run needs the owner's paid API key. This todo runs it and records the results.

## Context

- `tools/agent-eval/README.md`: how to pack the packages, export the key and run every task in both modes (`no-docs`, `docs`).
- [`agent-eval-baseline.md`](../../research/libraries/agent-eval-baseline.md): the page the results go in, with the task list already filled in.
- The L5a todo's risks: agent output varies, so run each task three times per mode and record pass counts and the cost.

## Scope

**In scope**

- Run every task three times in each mode with the agent and model pinned in `tools/agent-eval/eval.config.json`.
- Record the agent, model, date, Lamina PDF version, pass counts per task and mode, and the cost in the baseline page.
- Fix a checker or a task prompt that turns out wrong, in the same PR.

**Out of scope**

- Running the eval in CI or gating on it: not planned (it needs a paid API key and is non-deterministic).

## Tasks

- [ ] `cargo xtask wasm`, then `cargo xtask publish npm-pack`
- [ ] `ANTHROPIC_API_KEY=… node tools/agent-eval/run.mjs` three times
- [ ] Fill in `docs/research/libraries/agent-eval-baseline.md`; remove its "Not yet run" note

## Acceptance criteria

- [ ] `docs/research/libraries/agent-eval-baseline.md` records a run of every task with and without docs: agent, model, date, Lamina PDF version and per-task result

## Post-merge checks

None.

## Tests

None: the eval is a manual tool with its own checkers.

## Docs to update

- [ ] [shareable-libraries §9 L5](../../plans/shareable-libraries.md#9-to-dos): tick the baseline to-do

## Risks and open questions

- **Cost:** two modes, three runs and every task cost real money; the README estimates it. The owner decides how many runs.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
