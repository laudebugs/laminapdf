# U7b — The action controls: states, the hand cursor and hover fades

| Field | Value |
| --- | --- |
| ID | `U7b` |
| Status | `done` |
| Plan | [Motion and interaction states §7](../../plans/motion-and-states.md#7-todos) |
| Round / milestone | Round U (UI/UX) — U7 Motion and interaction states |
| Depends on | [U7a](../done/U7a-motion-and-state-tokens.md) |
| ADRs | Follows [0024](../../decisions/0024-app-shell-ux.md) (tokens in layers) |
| Branch | `feat/u7b-control-states-and-cursors` |
| PR title | `feat(ui): action controls show the hand, their states and quick fades` |
| Size | L (400–800) |
| PR | [#623](https://github.com/laudebugs/laminapdf/pull/623) |

## Goal

Apply plan §4.2 to the controls in `@laminapdf/ui` that run an action:

- the hand on every enabled one (the owner's choice, 2026-11-14), the arrow when disabled, `progress` when pending
- hover, pressed, selected + hover, focus-visible, disabled and pending styled in the plan's order
- hover fading in over 80 ms and out over 150 ms, press instant, hover that never sticks after a tap
- selection that never changes a control's width

## Context

- [Plan §2.1–2.2](../../plans/motion-and-states.md#2-what-the-apps-do-today-2026-11-14) and the reader [research](../../research/product/motion-and-states/README.md): `cursor: default` in `button.css:12`, `toggle.css:19`, `tabs.css:21`, `popover.css:38`; pressed missing on toggles and tabs; a selected toggle losing hover (`toggle.css:22-33`); tabs widening when selected
- `packages/ui/src/styles/` and `src/components/`; the gallery's `packages/ui/test/states.tsx`

## Scope

**In scope**

- Button (every variant, `danger-filled`'s hover and pressed included), IconButton (its pending state), ToggleButton, ToolToggle, ModeSwitcher, SegmentedTabs, menu items and options, StatusBarItem, ShortcutChip where it acts
- The cursor tokens on each; hard-coded cursors removed
- Hover from `data-hovered`; raw `:hover` in these components' CSS behind the guard
- Width-stable selected labels (plan §3, 7)
- Each supported state combination in `states.tsx`

**Out of scope**

- Checkboxes, radios, the switch, swatches, chips, the select, text fields, outline rows → U7i
- Overlays → U7c
- The reader's surfaces → U7d
- The sites → U7e

## Tasks

- [x] Cursors on each control
- [x] State styles in the plan's order
- [x] Hover fades with the tokens; press instant
- [x] Touch-safe hover
- [x] Width-stable selection
- [x] Gallery states (`states.tsx`), and a cursor check per control in the UI package's browser tests (the gallery's screenshots weren't regenerated: its Docker run times out on the machine that built this)

## Acceptance criteria

- [x] Every action control's computed cursor is `pointer` when enabled, `default` when disabled and `progress` when pending (checked in the UI package's browser tests, which CI runs in three engines; the gallery's Playwright tests run only in Docker and no workflow runs them)
- [x] Each shows a pressed state; a selected toggle still answers hover
- [x] No raw `:hover` left in these components' CSS outside the guard (the U7a baseline shrinks)
- [x] Selecting a tab or segment moves nothing beside it (each tab's and mode's width measured across selection, in the same browser tests)
- [x] axe passes in three engines; every label keeps 4.5:1 on its hover and pressed fill, light and dark (measured from the rendered colours with the layer composited, since the danger fills are mixes of tokens)

## Post-merge checks

None.

## Tests

- The gallery's visual tests (`cargo xtask gallery --test`; screenshots updated with `--update`, listed in the PR)
- The gallery's browser tests: cursor and the computed `transition-duration` per state
- The UI's Node tests, including the React Compiler pass

## Docs to update

- Design language §7 component sections where a state changes
- Plan §7: point this todo at `done/`

## Risks and open questions

- Gallery screenshots time out in Docker on the owner's Mac; say in the PR if they couldn't be refreshed locally

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] Every change people see tried in the built app, as a person uses it (CLAUDE.md, [bug triage §7](../../guidelines/bug-triage.md#7-prove-the-fix))
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
