# U7i — Choices, fields and rows: states, cursors and hover fades

| Field | Value |
| --- | --- |
| ID | `U7i` |
| Status | `ongoing` |
| Plan | [Motion and interaction states §7](../../plans/motion-and-states.md#7-todos) |
| Round / milestone | Round U (UI/UX) — U7 Motion and interaction states |
| Depends on | [U7a](../done/U7a-motion-and-state-tokens.md) |
| ADRs | Follows [0024](../../decisions/0024-app-shell-ux.md) (tokens in layers) |
| Branch | `feat/u7i-choice-and-field-states` |
| PR title | `feat(ui): choices, fields and rows follow the state rules` |
| Size | M (200–400) |
| PR | [#629](https://github.com/laudebugs/laminapdf/pull/629) |

## Goal

Apply plan §4.2 to the controls in `@laminapdf/ui` that choose, take input or list things, as U7b does for the action controls:

- checkboxes, radios, the switch, swatches and page-appearance chips get hover and pressed states, the hand, and the arrow when disabled
- the switch's knob slides by `transform` (and jumps under reduced motion, plan §3, 4)
- the select, text fields and text areas get their invalid, read-only and disabled looks in the plan's order
- outline rows get hover and pressed states

## Context

- The reader [research](../../research/product/motion-and-states/README.md): `choice.css:11` (`cursor: default`), no hover or pressed on checkbox, radio, switch and swatches (`choice.css`, `swatches.css`), the switch knob jumping, no read-only style on any field
- `packages/ui/src/styles/choice.css`, `swatches.css`, `field.css` (the select and the text fields), `outline-row.css` (its `cursor: default` on the `<a>`, lines 21 and 46)

## Scope

**In scope**

- Checkbox, RadioGroup, Switch, SwatchGroup, PageAppearanceChips, Select, TextInput, TextArea, OutlineRow
- The cursor tokens; hover fades; touch-safe hover
- Invalid (`danger` border and help, focus ring unchanged), read-only and disabled looks
- Each supported state combination in `states.tsx`

**Out of scope**

- Action controls → U7b
- Thumbnails and the reader's own rows → U7d

## Tasks

- [ ] Choices, with their `<label>` elements added to base.css's tap-highlight rule once they draw a pressed state
- [ ] The switch knob
- [ ] Fields
- [ ] Outline rows
- [ ] Gallery states and cursor checks

## Acceptance criteria

- [ ] In the gallery, every choice and row shows the hand when enabled and the arrow when disabled; text fields show the I-beam
- [ ] The switch knob slides; with reduced motion it jumps
- [ ] An invalid field's focus ring keeps its `focus` colour
- [ ] axe passes in three engines; contrast pairs pass

## Post-merge checks

None.

## Tests

- The gallery's visual and browser tests, as in U7b
- The UI's Node tests

## Docs to update

- Design language §7 component sections where a state changes
- Plan §7: point this todo at `done/`

## Risks and open questions

- The same gallery screenshot limit as U7b

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] Every change people see tried in the built app, as a person uses it (CLAUDE.md, [bug triage §7](../../guidelines/bug-triage.md#7-prove-the-fix))
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
