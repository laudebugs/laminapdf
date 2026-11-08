# Q3b — Diagrams for saving: the two save modes and how a save picks one

| Field | Value |
| --- | --- |
| ID | `Q3b` |
| Status | `ready` |
| Plan | [Docs diagrams §2](../../plans/docs-diagrams.md#2-where-a-diagram-helps) |
| Round / milestone | Round Q3 (docs diagrams) |
| Depends on | [Q3a](../done/Q3a-diagram-kit-and-first-diagram.md) |
| ADRs | Follows [0007](../../decisions/0007-save-planner.md) (what each mode does) |
| Branch | `feat/q3b-save-diagrams` |
| PR title | `feat(docs): diagrams for the two save modes and how a save picks one` |
| Size | S |
| PR | — |

## Goal

Saving is where readers hit the library's hardest rule: an incremental save keeps signatures valid and a full rewrite breaks them, and some files refuse one mode or both. Two diagrams on the save guide show the bytes each mode writes and the path from a file's state to a mode or an error code, so a reader who meets `would-invalidate-signatures` sees why and which option changes it.

## Context

- [Docs diagrams plan](../../plans/docs-diagrams.md) §3: the kit, the style and the checks Q3a built; every diagram here uses them.
- `apps/docs/src/content/docs/guides/save-a-pdf.mdx`: "Choose a save mode" and "When a file refuses a save", whose prose and option names the diagrams must match.
- [ADR 0007](../../decisions/0007-save-planner.md): the save planner's constraints and mode table, the source of truth for the decision flow.

## Scope

**In scope**

- Diagram 2, "Incremental save and full rewrite": two byte strips. The first holds the original bytes with a signature's coverage marked, then the appended revision; the second holds one rewritten block marked "signatures no longer valid".
- Diagram 3, "How a save picks its mode": the file's state (signed, repaired, encrypted and edited, unchanged) leading to `incremental`, `fullRewrite`, or the error it fails with (`would-invalidate-signatures` unless `invalidateSignatures: true`, `requires-rewrite`, `encrypted-base` unless `removeEncryption: true`).

**Out of scope** (and which todo covers it)

- The open-edit-save overview: Q3a.

## Tasks

- [ ] `components/diagrams/kit/Strip.astro`: a byte strip, segments with labels side by side, built from `Box` and tested like the rest of the kit.
- [ ] `components/diagrams/SaveModes.astro` (two `Strip`s) under "Choose a save mode".
- [ ] `components/diagrams/SaveDecision.astro` under "When a file refuses a save"; each refusal box names its error code in mono and links to nothing (the page's code table links).
- [ ] Check every option and code name against `packages/sdk/src` and `fixtures/codes.json`.

## Acceptance criteria

- [ ] Each diagram is built from the Q3a kit, renders in both themes (screenshots in the PR) and passes the lint, twin and browser tests.
- [ ] Each page's `.md` twin holds the caption and text description, and no SVG or JSX.
- [ ] Each diagram is under 8 KB gzipped (reported in the PR).
- [ ] `cargo xtask check docs` and the docs build pass.
- [ ] Every mode, option and error code in the diagrams exists under that name in `@laminapdf/sdk` and `fixtures/codes.json`.

## Post-merge checks

- [ ] The diagrams show on `next.docs.laminapdf.com` in both themes.

## Tests

The Q3a lint, twin and browser tests cover each new diagram once it is under `apps/docs/src/components/diagrams/`; no new test files unless a diagram needs a layout the kit lacks.

## Docs to update

The pages named in the tasks, and the plan's §9 to-do.

## Risks and open questions

- The decision flow can grow past a phone's width; plan §3.2's vertical layout applies.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
