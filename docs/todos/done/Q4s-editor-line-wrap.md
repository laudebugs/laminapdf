# Q4s — Unwrapped lines in "Edit and run", with a Wrap lines toggle

| Field | Value |
| --- | --- |
| ID | `Q4s` |
| Status | `done` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | [Q4q](./Q4q-resizable-page-beside-the-code.md) |
| ADRs | [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (the CSP the editor runs under) |
| Branch | `claude/inspiring-lamport-q8469n` (this session's branch) |
| PR title | `feat(docs): unwrapped lines in "Edit and run", with a Wrap lines toggle` |
| Size | S |
| PR | [#558](https://github.com/laudebugs/laminapdf/pull/558) |

## Goal

The "Edit and run" editor wraps every long line, so a line of code can break in the middle of an expression and its indentation no longer shows the code's shape. The owner asked for the editor to keep lines unwrapped by default, scrolling sideways as an editor does, with a button in the runner's bar to wrap them when a reader prefers.

## Context

- `apps/docs/runner/editor.ts`: the editor (CodeMirror), built today with `EditorView.lineWrapping` always on.
- `apps/docs/src/components/Example.astro`: the runner's bar (Run, Reset, Download PDF) and its styles.
- `apps/docs/e2e/runner.e2e.test.ts`: the runner's e2e suite.
- [Q4r](https://github.com/laudebugs/laminapdf/pull/549) (open): a "Wrap long lines" button on the read-only code blocks, a Format button at the bar's far end, and a `wrap-lines` icon. This todo leaves the code blocks alone and keeps the editor's setting its own.

## Scope

**In scope**

- The editor starts with its lines unwrapped.
- A "Wrap lines" toggle in the runner's bar, after Reset, pressed (`aria-pressed`) while lines wrap. Pressing it again unwraps them. Reset leaves the setting as it is.

**Out of scope**

- The read-only code blocks' wrapping (Q4r).
- An icon for the toggle: Q4r adds `wrap-lines`; once it merges, the toggle can take it.
- Remembering the setting across pages.

## Tasks

- [x] `editor.ts`: line wrapping in a CodeMirror compartment, off at first, switched by the toggle.
- [x] `Example.astro`: the toggle in the bar, its pressed look.
- [x] `runner.e2e.test.ts`: lines unwrapped at first (the editor scrolls sideways), wrapped after a press, unwrapped after a second.
- [x] The docs examples plan's editor notes.

## Acceptance criteria

- [x] The editor opens unwrapped: a long line scrolls sideways instead of wrapping.
- [x] The toggle wraps the lines and says it is pressed; a second press unwraps them.
- [x] No CSP violation is reported.

## Post-merge checks

None.

## Tests

The docs e2e suite (`runner.e2e.test.ts`) in Chromium, Firefox and WebKit (`cargo xtask pdf-runtimes <browser>`).

## Docs to update

The [docs examples plan](../../plans/docs-examples.md) (§2's editor, §4) and the [docs redesign plan](../../plans/docs-redesign.md) (§4).

## Risks and open questions

- Q4r's PR touches the same bar and plan tables; whichever merges second takes the other's lines.

## Definition of done

- [x] Claimed: the work PR moved this file to `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
