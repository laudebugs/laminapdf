# U7d — The reader's surfaces: thumbnails, pages, panels and tool cursors

| Field | Value |
| --- | --- |
| ID | `U7d` |
| Status | `ready` |
| Plan | [Motion and interaction states §7](../../plans/motion-and-states.md#7-todos) |
| Round / milestone | Round U (UI/UX) — U7 Motion and interaction states |
| Depends on | [U7b](../ongoing/U7b-control-states-and-cursors.md), [U7i](./U7i-choice-and-field-states.md) |
| ADRs | Follows [0024](../../decisions/0024-app-shell-ux.md) (tokens in layers) |
| Branch | `feat/u7d-reader-surfaces-motion` |
| PR title | `feat(web): thumbnails, panels and tool cursors follow the state rules` |
| Size | M (200–400) |
| PR | — |

## Goal

Bring the reader's surfaces that aren't plain controls under the plan's §3 and §4, then lock the cursor rule with a test over the built reader:

- thumbnails get hover and pressed states and fade from placeholder to picture; pages fade in, and from their low-resolution render to the sharp one (plan §3, 10)
- the inspector opens by `transform` and `opacity` over `duration-enter-panel` and closes over `duration-exit-panel` (200 and 150 ms), in a dock that takes its width at once, and the phone top bar leaves by `translateY`; both fade under reduced motion
- a creation tool's cursor wins over the text layer's I-beam, and the hand tool shows `grab` and `grabbing`
- the reader's raw `:hover` rules (document tabs and their close button, history rows, threads, the rail handle, properties rows, reader blocks) move behind the guard, and recent rows take the cursor tokens
- a test fails the build on any control in the built reader with the wrong cursor

## Context

- [Plan §2](../../plans/motion-and-states.md#2-what-the-apps-do-today-2026-11-14) and the reader [research](../../research/product/motion-and-states/README.md): `apps/web/src/app.css:83-113` (thumbnails), `shell.css:58-68` (inspector), `phone.css:26-36`, `text-layer.ts:275` vs `tools.ts:157`, `document-tabs.css:40,99-106`, `history.css:65`, `threads.css:54,76,79-80`, `recent.css:43` (cursor), `shell.css:26`, `properties.css:68`, `reader.css:817`

## Scope

**In scope**

- Thumbnails' states and fades
- Pages' fade-in and the low-resolution to sharp cross-fade
- Inspector and phone top bar motion without layout animation
- Tool cursors over the text and link layers (the tool's cursor on the layer root wins; spans keep the I-beam only for text selection)
- The reader's raw `:hover` rules, all of them, including `document-tabs.css`'s (U7f and U7g change the tabs' motion and drag, not their hover) and `properties.css`'s row hover that shows its icon button
- The reader's own buttons outside U7b's action controls, onto its shared states (`states.css`): `.paper-zoom-percent`, `.paper-commands-button`, `.paper-password-show`, `.paper-document-tabs-menu`, `.paper-bottom-bar-button`
- The built-reader cursor test (plan §6)

**Out of scope**

- The tab strip's motion and reserved space → U7f; tab dragging → U7g

## Tasks

- [ ] Thumbnails
- [ ] Page fades
- [ ] Inspector and phone bar
- [ ] Tool cursors
- [ ] Raw `:hover` rules
- [ ] The cursor test

## Acceptance criteria

- [ ] The Link tool shows its crosshair over page text; text selection still shows the I-beam
- [ ] Opening the inspector moves nothing in the page view during its animation (the canvas width is set on the first frame)
- [ ] A page fades in rather than appearing in one frame; with reduced motion it still fades
- [ ] The cursor test passes, and fails if one button's cursor is changed back to `default`

## Post-merge checks

None.

## Tests

- The cursor test (Chromium, Firefox, WebKit)
- The viewer's browser tests for the text layer and page rendering
- The reader's e2e suite

## Docs to update

- Plan §7: point this todo at `done/`

## Risks and open questions

- The inspector's dock width is part of the shell's layout state; changing how it opens must keep the saved rail and inspector widths
- A page fade must not delay the first readable frame: the page shows at full opacity as soon as its sharp render is ready

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] Every change people see tried in the built app, as a person uses it (CLAUDE.md, [bug triage §7](../../guidelines/bug-triage.md#7-prove-the-fix))
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
