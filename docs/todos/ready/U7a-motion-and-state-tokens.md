# U7a — Motion, cursor and state tokens, and the rules that go with them

| Field | Value |
| --- | --- |
| ID | `U7a` |
| Status | `ready` |
| Plan | [Motion and interaction states](../../plans/motion-and-states.md) |
| Round / milestone | Round U (UI/UX) — U7 Motion and interaction states |
| Depends on | None |
| ADRs | Follows 0024 (tokens in layers), 0010 (capability flags) |
| Branch | `feat/u7a-motion-and-state-tokens` |
| PR title | `feat(tokens): motion, cursor and state rules for Paper` |
| Size | M (200–600) |
| PR | — |

## Goal

Give every component author what the [motion and states plan](../../plans/motion-and-states.md) §4 decides: duration, easing and cursor tokens in `@laminapdf/tokens`, the state model and its order of precedence in the design language's §7.0, one meaning for reduced motion, a way to suppress transitions while the look switches, and static checks that keep literal timings, `transition: all` and unguarded `:hover` out. No component changes yet: U7b–U7e apply them.

## Context

- [Plan §3 and §4](../../plans/motion-and-states.md#3-decisions): the owner's four choices (2026-11-14) and the tokens, state model and motion rules
- `packages/tokens/tokens.json`'s `motion` group and `scripts/generate.ts`
- [Design language §7.0](../../design/paper/design-language.md) (the state table, hover, pressed and disabled still proposed) and its accessibility checklist
- `packages/ui/src/styles/settings.css` (Settings › Reduce motion's blanket override) and the tokens-only check in `packages/ui/test/`

## Scope

**In scope**

- The tokens of plan §4.1 (durations, easings, cursors), generated into `--paper-*` and `tokens.ts`, with their reduced-motion values
- Design language §7.0: the state model and precedence (plan §4.2), hover, pressed and disabled no longer proposed, the cursor rule; the checklist's reduced-motion line rewritten to plan §3, 4
- `data-motion-suppressed` on `:root` for one frame around theme, accent, page appearance, density and direction switches (plan §3, 5)
- Settings › Reduce motion follows the same rule as the system preference: movement off, fades kept
- Static checks: no literal duration, easing or cursor value, no `transition: all`, no `:hover` outside `@media (hover: hover) and (pointer: fine)` in `@laminapdf/ui`'s and the sites' CSS, with a baseline file listing today's offenders that only shrinks

**Out of scope**

- Changing components → U7b, U7c, U7d
- The sites' own CSS → U7e

## Tasks

- [ ] Tokens and generator
- [ ] Design language §7.0 and the checklist
- [ ] `data-motion-suppressed` and its hook in the theme and appearance switches
- [ ] Settings › Reduce motion's rule
- [ ] The static checks and their baseline

## Acceptance criteria

- [ ] `--paper-duration-state-in` and the other plan §4.1 tokens exist in light, dark and high contrast, and `tokens.ts` exports them
- [ ] Switching theme or accent changes no colour gradually (measured in a browser: the root's computed `transition-duration` is 0 for the frame)
- [ ] Settings › Reduce motion on: a hover still fades; a transform transition resolves to none
- [ ] The checks fail on a new literal `120ms` or `transition: all`, and pass on main's files through the baseline

## Post-merge checks

None.

## Tests

- The tokens package's tests: coverage of the new tokens, reduced-motion values
- The check's own tests on fixtures
- The UI's browser test for reduced motion (`packages/ui/test/browser`) extended to fades kept, movement off

## Docs to update

- Design language §7.0 and checklist
- `packages/tokens/README.md`
- Plan §7: point this todo at `done/`

## Risks and open questions

- The baseline file must list every current offender exactly, or the check blocks unrelated PRs

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] Every change people see tried in the built app, as a person uses it (CLAUDE.md, [bug triage §7](../../guidelines/bug-triage.md#7-prove-the-fix))
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
