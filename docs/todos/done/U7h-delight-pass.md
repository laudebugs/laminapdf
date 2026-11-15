# U7h — The round's close: a view transition, feedback and a pass over every screen

| Field | Value |
| --- | --- |
| ID | `U7h` |
| Status | `done` |
| Plan | [Motion and interaction states §7](../../plans/motion-and-states.md#7-todos) |
| Round / milestone | Round U (UI/UX) — U7 Motion and interaction states |
| Depends on | [U7b](./U7b-control-states-and-cursors.md), [U7c](./U7c-overlay-enter-and-exit.md), [U7d](./U7d-reader-surfaces-motion.md), [U7e](./U7e-sites-states-and-motion.md), [U7f](./U7f-desktop-chrome-motion.md), [U7g](./U7g-pointer-tab-drag.md), [U7i](./U7i-choice-and-field-states.md) |
| ADRs | Follows [0024](../../decisions/0024-app-shell-ux.md) (tokens in layers) |
| Branch | `feat/u7h-delight-pass` |
| PR title | `feat(ui): a view transition for the Reader toggle, and clearer feedback` |
| Size | M (200–400) |
| PR | [#642](https://github.com/laudebugs/laminapdf/pull/642) |

## Goal

Close Round U7:

- a same-document view transition when the Reader toggle switches between pages and the reading column (`duration-view`, 250 ms; a cross-fade under reduced motion)
- confirmation that a copy or an undo happened: a short toast, or the History tab's step lighting up
- one by-hand pass over every screen in the screen inventory and both sites, and over tab dragging on each desktop OS, against the plan, with whatever it finds fixed or written up as todos

## Context

- [Plan §1 and §4.3](../../plans/motion-and-states.md#43-rules-for-motion)
- [Screen inventory](../../design/paper/screens.md)

## Scope

**In scope**

- The Reader toggle's view transition
- Copy and undo feedback
- The by-hand pass and its findings

**Out of scope**

- Brand motion → the owner, after this todo (plan §8, 1)

## Tasks

- [x] View transition
- [x] Feedback
- [x] The pass: the web reader and both sites in Chromium and WebKit, with and without reduced motion, at desktop and phone width; small findings fixed here, the rest written up as [U7j](../ready/U7j-reader-pass-fixes.md) and [U7k](../ready/U7k-sites-pass-fixes.md). The desktop app and tab dragging on each OS moved to [U7l](../ready/U7l-desktop-pass.md): they need U7f merged and a Windows and a Linux machine by hand. Playwright's Firefox wouldn't launch outside Vitest on the Mac, so U7j and U7k check Firefox

## Acceptance criteria

- [x] Switching to the reading column and back cross-fades in Chromium, Firefox and Safari (by hand in Chromium and WebKit; Firefox in the browser tests), and is instant where view transitions are missing
- [x] Every screen in the inventory and every page type of both sites checked, with a list in the PR: the desktop screens are in U7l, and the PR names the four web reader screens left unchecked (R17, O16, P02 and phone text selection), which U7j covers

## Post-merge checks

None.

## Tests

- A browser test of the view transition with and without reduced motion

## Docs to update

- Plan: Status `Done`, §7 ticked
- [Paper design plan](../../plans/paper-design.md) and [UI/UX plan](../../plans/ui-ux.md): Round U7 done

## Risks and open questions

- A view transition snapshots the page; on a large document the reading column's first frame must be ready, or the transition shows a blank

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] Every change people see tried in the built app, as a person uses it (CLAUDE.md, [bug triage §7](../../guidelines/bug-triage.md#7-prove-the-fix))
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
