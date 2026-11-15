# U7c — Overlays enter and leave

| Field | Value |
| --- | --- |
| ID | `U7c` |
| Status | `ready` |
| Plan | [Motion and interaction states](../../plans/motion-and-states.md) |
| Round / milestone | Round U (UI/UX) — U7 Motion and interaction states |
| Depends on | [U7a](./U7a-motion-and-state-tokens.md) |
| ADRs | Follows 0024 (tokens in layers), 0010 (capability flags) |
| Branch | `feat/u7c-overlay-enter-and-exit` |
| PR title | `feat(ui): menus, dialogs and toasts fade in and out` |
| Size | M (200–600) |
| PR | — |

## Goal

Make overlays enter and leave as the owner chose (2026-11-14): menus, popovers, tooltips, link previews and mini toolbars fade in with a scale from 0.97 at their anchor over 120 ms and out over 80 ms; dialogs, toasts, sheets, the rail drawer and its scrim over 200 ms in and 150 ms out; the command palette stays instant. An overlay that is leaving lets clicks and keys through from its first frame.

## Context

- [Plan §3, 3 and 6](../../plans/motion-and-states.md#3-decisions); §2.3: no overlay uses `data-entering` or `data-exiting` today
- `packages/ui/src/overlays/` (Popover, Menu, Tooltip, Dialog, ToastRegion, BottomSheet, SelectionToolbar, LinkPreview, CommandPalette) and `src/shell/` (the rail drawer and scrim)

## Scope

**In scope**

- React Aria's `data-entering` and `data-exiting` on each overlay, with `transform-origin` at the anchor
- The tokens of plan §4.1; reduced motion: opacity only
- Exiting overlays `inert` and `pointer-events: none`
- The drawer slides by `transform`, the scrim fades
- A toast leaves the way it came

**Out of scope**

- Same-document view transitions → U7h
- The desktop's own notices → U7f

## Tasks

- [ ] Small overlays
- [ ] Panels and sheets
- [ ] The drawer and scrim
- [ ] Exit never blocks input
- [ ] Reduced motion

## Acceptance criteria

- [ ] Opening and closing each overlay in the built reader shows the fade and scale; with reduced motion, a fade only
- [ ] A second click during a menu's exit reaches what is under it
- [ ] The palette opens and closes in the same frame as its shortcut

## Post-merge checks

None.

## Tests

- A browser test per overlay type: classes and computed styles during enter and exit, with and without reduced motion
- The existing overlay tests (layer stack, Escape, focus return) still pass

## Docs to update

- Design language §7's overlay sections
- Plan §7: point this todo at `done/`

## Risks and open questions

- React Aria keeps an exiting overlay in the DOM until its animation ends: an animation that never ends (a hidden tab) must not leave a ghost; test with `animationend` missing

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] Every change people see tried in the built app, as a person uses it (CLAUDE.md, [bug triage §7](../../guidelines/bug-triage.md#7-prove-the-fix))
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
