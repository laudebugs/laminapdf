# U7j — The reader's pass findings: sheets that slide, page tiles with states, and the polish

| Field | Value |
| --- | --- |
| ID | `U7j` |
| Status | `ready` |
| Plan | [Motion and interaction states §7](../../plans/motion-and-states.md#7-todos) |
| Round / milestone | Round U (UI/UX) — U7 Motion and interaction states |
| Depends on | [U7h](../done/U7h-delight-pass.md) |
| ADRs | Follows [0024](../../decisions/0024-app-shell-ux.md) (tokens in layers) |
| Branch | `fix/u7j-reader-pass-fixes` |
| PR title | `fix(ui): phone sheets slide in and out, page tiles take the state model` |
| Size | M (200–400) |
| PR | — |

## Goal

Fix what U7h's by-hand pass over the web reader found against the plan: the phone sheets don't slide up and the Pages sheet has no exit, Organise's page tiles skip the state model, and five smaller things: three shifts in one frame, a scrim's cursor and the page view's focus ring.

## Context

- [Plan §3 and §4](../../plans/motion-and-states.md#3-decisions): the rules each finding breaks
- U7h's PR lists the pass, screen by screen, in Chromium and WebKit with and without reduced motion

## Scope

**In scope**

- **Phone sheets (F04, R18):** the Pages and More sheets sit in place from their first frame; only the scrim fades. React Aria's `data-entering` never shows on `.paper-sheet-overlay`, so the slide in `overlay-motion.css` never runs. Fix: start the sheet from a `@starting-style`, as the toast and drawer do (plan §3, decision 3)
- **The Pages sheet** flashes (opacity 1, 0.7, 0, then in) on opening in Chromium. It also leaves in one frame on Escape, a scrim tap or Close; the More sheet fades out over exit-panel (decision 6)
- **Organise's page tiles (O06):** `.paper-page-tile` sets `background` in `organise.css` and isn't in `states.css`'s list, so hover snaps (`transition: all 0s`), there's no pressed state and a selected tile shows no hover (decision 2; §4.2, rule 3)
- **Polish:**
  - The first edit adds "1 unsaved change" and Save, pushing the top bar about 100 px in one frame (decision 7)
  - The Properties dialog is 189 px tall on its first frame and 727 px the next, mid scale-in, and changes height between its tabs
  - The offline strip pushes the page down 44 px in one frame
  - The rail drawer's scrim shows the arrow (`shell.css`); decide whether a scrim takes the hand
  - Keyboard focus on the page view (`.lamina-viewer`) shows the browser's ring, not Paper's (§4.2, rule 5)
  - During the Reader toggle's 250 ms view transition the page takes no clicks: the browser hit-tests the root (U7h's e2e waits it out; `pointer-events: none` on `::view-transition` didn't help in Chromium)

**Out of scope**

- The sites' findings → [U7k](./U7k-sites-pass-fixes.md); the desktop pass → [U7l](./U7l-desktop-pass.md)

## Tasks

- [ ] Sheets: the slide in and the exit, both sheets, both motion settings
- [ ] Page tiles in the state model
- [ ] The polish items, each fixed or, where the plan doesn't decide, asked as a GitHub issue
- [ ] Check what U7h's pass left out: R17 (tablet), O16, P02, phone text selection
- [ ] Check Firefox too: U7h's pass couldn't launch Playwright's Firefox outside Vitest on the Mac (`@laminapdf/test-utils`' `firefoxEnv` is what the suites use)

## Acceptance criteria

- [ ] The phone sheets slide up and down (fade only under reduced motion) in Chromium, Firefox and WebKit
- [ ] Page tiles hover, press and show hover when selected, with the state transitions
- [ ] None of the three shifts happens in one frame, the drawer's scrim has the cursor decided for it, and the page view shows Paper's ring

## Post-merge checks

None.

## Tests

- Browser tests in `packages/ui/test/browser/` (sheet motion beside `overlay-motion.browser.test.tsx`, tile states beside `states.browser.test.tsx`)

## Docs to update

- Plan §7: this row

## Risks and open questions

- Reserving the top bar's room for the save state may need a design call; ask before changing the layout

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] Every change people see tried in the built app, as a person uses it (CLAUDE.md, [bug triage §7](../../guidelines/bug-triage.md#7-prove-the-fix))
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
