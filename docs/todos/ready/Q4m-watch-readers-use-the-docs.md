# Q4m — Watch developers use the docs, and read the agent eval

| Field | Value |
| --- | --- |
| ID | `Q4m` |
| Status | `ready` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | [L5b](./L5b-agent-eval-baseline.md) |
| ADRs | [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (no analytics) |
| Branch | `docs/q4m-docs-sessions` |
| PR title | `docs(research): what developers did when following the docs` |
| Size | M (sessions and notes, little code) |
| PR | — |

## Goal

The docs collect no analytics, by design, so nothing says where readers get stuck. The research's evidence on reading behaviour rests on one study of eleven people. This todo watches five to eight developers who haven't used Lamina PDF follow the quick starts and the reading path, notes where they stop, search or go wrong, and reads the agent eval's results beside them. Its findings decide Q4g.

## Context

- [Docs research: recommendation 13](../../research/docs-site/developer-docs.md#recommendations-in-order) and its conclusion.
- L5b's agent eval results (`tools/agent-eval/`).
- The owner's budget for paid sessions.

## Scope

**In scope**

- A session script: tasks (make a first PDF, extract text from a given file, fix a file that won't save), think-aloud, no help.
- Five to eight sessions, recorded with consent, notes kept without personal data.
- A note in `docs/research/docs-site/` with what readers did and what to change, and the plan updated.

**Out of scope**

- Building anything in the docs (later todos do).

## Tasks

- [ ] Write the tasks and the script; agree the budget with the owner.
- [ ] Recruit developers across TypeScript, Rust and the command line.
- [ ] Run and write up the sessions; read L5b's results.
- [ ] Decide with the docs track whether Q4g goes ahead and what it changes.

## Acceptance criteria

- [ ] A research note with each session's path and the problems seen, counted.
- [ ] A decision on Q4g written in the plan.

## Post-merge checks

None.

## Tests

None.

## Docs to update

`docs/research/docs-site/README.md`; the docs redesign plan.

## Risks and open questions

- Five sessions find the big problems but not their frequency; say so in the note.
- Recordings are personal data; keep only notes, and delete recordings after the write-up.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
