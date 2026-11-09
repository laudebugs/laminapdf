# Q3f — Diagrams for the Web Worker, errors and warnings, and transactions

| Field | Value |
| --- | --- |
| ID | `Q3f` |
| Status | `ongoing` |
| Plan | [Docs diagrams §2](../../plans/docs-diagrams.md#2-where-a-diagram-helps) |
| Round / milestone | Round Q3 (docs diagrams) |
| Depends on | [Q3a](../done/Q3a-diagram-kit-and-first-diagram.md) |
| ADRs | Follows [0006](../../decisions/0006-op-log-and-undo.md), [0026](../../decisions/0026-error-and-diagnostic-contract.md) |
| Branch | `feat/q3f-worker-errors-undo` |
| PR title | `feat(docs): diagrams for the Web Worker, errors and warnings, and undo` |
| Size | M |
| PR | [#346](https://github.com/laudebugs/laminapdf/pull/346) |

## Goal

Three concept pages explain behaviour readers get wrong in their first week: what a Web Worker call can cancel, how an error differs from a warning, and what one undo takes back. Each gets one small diagram next to the prose that explains it.

## Context

- [Docs diagrams plan](../../plans/docs-diagrams.md) §3: the kit, the style and the checks Q3a built; every diagram here uses them.
- `apps/docs/src/content/docs/guides/keep-the-page-responsive.mdx`: "Call the same methods, awaited" and "Cancel a call".
- `apps/docs/src/content/docs/concepts/errors-and-warnings.mdx`: "Errors stop the call" and "Every surface has the same fields".
- `apps/docs/src/content/docs/guides/edit-a-pdf.mdx`: "Group edits into a transaction" and "Undo and redo".
- [ADR 0026](../../decisions/0026-error-and-diagnostic-contract.md) (the error object) and [ADR 0006](../../decisions/0006-op-log-and-undo.md) (transactions and undo).

## Scope

**In scope**

- Diagram 8, "Lamina PDF in a Web Worker": the page and `connect()`'s client, messages to the Web Worker that runs `@laminapdf/sdk`, answers as Promises with transferred bytes; cancelling: an `AbortSignal` drops a queued call, a running `save` stops at its next step, any other running call finishes.
- Diagram 9, "Errors and warnings": a call either stops with a `LaminaPdfError` or completes with warnings on `doc.warnings`; both share `code`, `kind`, `message`, `hint`, `location`, `data` and `docsUrl`, with the document's own text only in `data`.
- Diagram 10, "Transactions and undo": three transactions as units on a timeline; one `undo` steps back a whole transaction; `redo` steps forward.

**Out of scope** (and which todo covers it)

- The viewer's own worker and tiles: a later viewer page (plan §2).

## Tasks

- [x] `components/diagrams/WorkerCalls.astro`, `ErrorsAndWarnings.astro` and `UndoTimeline.astro`, each placed in its section above.
- [x] Check the cancel rules against `packages/sdk/src/worker/` and the field names against `packages/sdk/schemas/error.schema.json`.

## Acceptance criteria

- [x] Each diagram is built from the Q3a kit, renders in both themes (screenshots in the PR) and passes the lint, twin and browser tests.
- [x] Each page's `.md` twin holds the caption and text description, and no SVG or JSX.
- [x] Each diagram is under 8 KB gzipped (reported in the PR).
- [x] `cargo xtask check docs` and the docs build pass.
- [x] The cancel rules and the error fields match the SDK's behaviour and schema.

## Post-merge checks

- [ ] The diagrams show on `next.docs.laminapdf.com` in both themes.

## Tests

The Q3a lint, twin and browser tests cover each new diagram once it is under `apps/docs/src/components/diagrams/`; no new test files unless a diagram needs a layout the kit lacks.

## Docs to update

The pages named in the tasks, and the plan's §9 to-do.

## Risks and open questions

- The worker diagram is a sequence over time; if the kit's lanes can't show it at phone width, it gets the vertical layout of plan §3.2.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
