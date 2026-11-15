# U7a — Motion, cursor and state tokens, and the rules that go with them

| Field | Value |
| --- | --- |
| ID | `U7a` |
| Status | `done` |
| Plan | [Motion and interaction states §7](../../plans/motion-and-states.md#7-todos) |
| Round / milestone | Round U (UI/UX) — U7 Motion and interaction states |
| Depends on | None |
| ADRs | Follows [0024](../../decisions/0024-app-shell-ux.md) (tokens in layers) |
| Branch | `feat/u7a-motion-and-state-tokens` |
| PR title | `feat(tokens): motion, cursor and state rules for Paper` |
| Size | M (200–400) |
| PR | [#622](https://github.com/laudebugs/laminapdf/pull/622) |

## Goal

Give every component author what the plan's §3 and §4 decide, before any component changes (later U7 todos apply them):

- duration, easing and cursor tokens in `@laminapdf/tokens`
- the state model and its order of precedence in the design language's §7.0, and one meaning for reduced motion
- a way to suppress transitions while the look switches, and the iOS tap highlight cleared once for every control
- static checks that keep literal timings, `transition: all` and unguarded `:hover` out

## Context

- [Plan §3 and §4](../../plans/motion-and-states.md#3-decisions): the owner's four choices (2026-11-14) and the tokens, state model and motion rules; the [research](../../research/product/motion-and-states/README.md)
- `packages/tokens/tokens.json`'s `motion` group and `scripts/generate.ts`
- [Design language §7.0](../../design/paper/design-language.md) (the state table, hover, pressed and disabled still proposed) and its accessibility checklist
- `packages/ui/src/styles/settings.css:217-229` (Settings › Reduce motion's blanket override), the tokens-only check in `packages/ui/test/`, and the gallery's motion URL switch (`apps/gallery`)

## Scope

**In scope**

- The tokens of plan §4.1 in the semantic layer, generated into `--paper-*` and `tokens.ts`, with their reduced-motion values
- Design language §7.0: the state model and precedence (plan §4.2), the cursor rule, hover, pressed and disabled no longer proposed; the checklist's reduced-motion line rewritten to plan §3, 4
- `data-motion-suppressed` on `:root` for one frame around theme, accent, page appearance, density and direction switches (plan §3, 5)
- Settings › Reduce motion and the gallery's motion switch follow the system preference's rule: movement off, fades kept
- `-webkit-tap-highlight-color: transparent` for every control that draws its own pressed state (plan §4.3)
- Static checks with a baseline file of today's offenders that only shrinks

**Out of scope**

- Changing components → U7b, U7i, U7c, U7d
- The sites' own CSS → U7e

## Tasks

- [x] Add the tokens of plan §4.1 to `tokens.json` (DTCG `duration` and `cubicBezier`), generate them, and give each its reduced-motion value in the generator's reduced-motion block
- [x] Rewrite §7.0's state table in the plan's order and add the cursor rule; rewrite the checklist's reduced-motion line
- [x] Add `data-motion-suppressed` and set it, from the switch until the frame after the new look draws, in the theme, accent, appearance, density and direction setters
- [x] Change Settings › Reduce motion's CSS from `transition: none !important` to removing transforms and size changes only; make the gallery's motion switch do the same
- [x] Clear the tap highlight in the base layer for buttons, `[role=button]`, tabs, options, menu items and rows
- [x] Extend the tokens-only check: literal durations, easings and cursor values, `transition: all`, and `:hover` outside `@media (hover: hover) and (pointer: fine)`, in `packages/ui/src` and the sites' CSS; write the baseline

## Acceptance criteria

- [x] `--paper-duration-state-in` and the other plan §4.1 tokens exist in light, dark and high contrast, and `tokens.ts` exports them
- [x] Switching theme or accent changes no colour gradually (measured in a browser: the root's computed `transition-duration` is 0 for that frame)
- [x] Settings › Reduce motion on: a hover still fades; a transform transition resolves to none
- [x] The checks fail on a new literal `120ms` or `transition: all`, and pass on main's files through the baseline

## Post-merge checks

None.

## Tests

- The tokens package's tests: coverage of the new tokens and their reduced-motion values
- The check's own tests on fixtures
- The UI's browser test for reduced motion (`packages/ui/test/browser`), extended to fades kept and movement off

## Docs to update

- Design language §7.0 and checklist
- `packages/tokens/README.md`
- Plan §7: point this todo at `done/`

## Risks and open questions

- The baseline file must list every current offender exactly, or the check blocks unrelated PRs

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] Every change people see tried in the built app, as a person uses it (CLAUDE.md, [bug triage §7](../../guidelines/bug-triage.md#7-prove-the-fix))
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
